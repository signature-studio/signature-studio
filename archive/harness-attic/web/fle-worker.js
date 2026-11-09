'use strict';
// Web Worker that runs the 6-module flengine6.wasm (lift/harness/two/build_nmod_wasm.sh, linked for the web by link-web.sh) the way
// lift/harness/two/run_node_nmod.cjs runs it under node: same arguments (<FLEngine dll> <5 rebased module files joined with ':'>),
// FLE_VFS_SEED / FLE_VFS_SEED_ROOT in the environment, FLE_WASM_FAST_RETADDR=1 as `fast`. Under node the guest reads the host files
// (NODERAWFS); here the same files are fetched over HTTP and written to the in-memory file system (MEMFS) before main().
// Messages to the page: {type:'mark',name,t}, {type:'stdout'|'stderr',line}, {type:'done',...}, {type:'error',message}.
// Message from the page: {cmd:'run', glue, wasm, dataBase, root, fast, mode, enc, cache, rate}
const T0 = performance.now();
const marks = {};
const post = (m) => self.postMessage(m);
const mark = (name, extra) => {
  const t = +(performance.now() - T0).toFixed(1);
  marks[name] = t;
  post(Object.assign({ type: 'mark', name, t }, extra));
  return t;
};
const heap = () => (self.performance && performance.memory)
  ? { used: performance.memory.usedJSHeapSize, total: performance.memory.totalJSHeapSize, limit: performance.memory.jsHeapSizeLimit } : null;

const MODULES = ['dsp_ippv2_x64.2000000.dll', 'Blend2D.4700000.dll', 'quickfontcache_x64.1d00000.dll', 'freetype_x64.4b00000.dll', 'ILTools_x64.6000000.dll'];
const FLE = 'FLEngine_x64.dll';
const SEED_TSV = 'seed/vfs-seed-fonts.tsv';
const SEED_FILES = ['seed/lift/out-inventory/extracted/fonts/ILCursors.ilfont', 'seed/lift/out-inventory/extracted/fonts/ILGlyphsEx.ilfont'];

async function run(c) {
  const root = c.root || '/data';
  const stdout = [];
  const stderr = [];
  let exitCode = null;
  let instance = null;
  let wasmModule = null;
  const wasmDetail = {};

  importScripts(c.glue);
  mark('glue_loaded');

  const names = [FLE, ...MODULES, SEED_TSV, ...SEED_FILES];
  const t0 = performance.now();
  const bufs = await Promise.all(names.map(async (n) => {
    const r = await fetch(c.dataBase + n, { cache: 'no-store' });
    if (!r.ok) throw new Error(n + ': HTTP ' + r.status);
    return new Uint8Array(await r.arrayBuffer());
  }));
  const bytes = bufs.reduce((s, b) => s + b.length, 0);
  mark('data_fetched', { files: names.length, bytes, ms: +(performance.now() - t0).toFixed(1) });

  const opts = {
    arguments: [root + '/' + FLE, MODULES.map((m) => root + '/' + m).join(':')],
    print: (line) => { stdout.push(line); post({ type: 'stdout', line }); },
    printErr: (line) => { stderr.push(line); post({ type: 'stderr', line }); },
    onExit: (code) => { exitCode = code; mark('main_end'); },
    onAbort: (what) => { post({ type: 'stderr', line: 'onAbort: ' + what }); },
    onRuntimeInitialized: () => mark('main_start', { heap: heap() }),
    preRun: [() => {
      const FS = opts.FS;
      const put = (guest, data) => {
        FS.mkdirTree(guest.slice(0, guest.lastIndexOf('/')));
        FS.writeFile(guest, data, { canOwn: true });
      };
      names.forEach((n, i) => put(root + '/' + n, bufs[i]));
      opts.ENV.FLE_VFS_SEED = root + '/' + SEED_TSV;
      opts.ENV.FLE_VFS_SEED_ROOT = root + '/seed';
      mark('memfs_ready');
    }],
    instantiateWasm(imports, done) {
      if (c.fast) imports.env.emscripten_return_address = () => 0;
      (async () => {
        const u = new URL(c.wasm);
        if (c.enc && c.enc !== 'identity') u.searchParams.set('enc', c.enc);
        if (c.cache) u.searchParams.set('cache', '1');
        if (c.rate) u.searchParams.set('rate', String(c.rate));
        const url = u.href;
        const fc = c.cache ? 'default' : 'no-store';
        const s = performance.now();
        mark('wasm_start', { mode: c.mode, enc: c.enc || 'identity', heap: heap() });
        if (c.mode === 'compile') {
          wasmModule = await WebAssembly.compileStreaming(fetch(url, { cache: fc }));
          wasmDetail.compile_ms = +(performance.now() - s).toFixed(1);
          mark('wasm_compiled', { heap: heap() });
          const s2 = performance.now();
          instance = await WebAssembly.instantiate(wasmModule, imports);
          wasmDetail.instantiate_ms = +(performance.now() - s2).toFixed(1);
        } else if (c.mode === 'buffer') {
          const buf = await (await fetch(url, { cache: fc })).arrayBuffer();
          wasmDetail.download_ms = +(performance.now() - s).toFixed(1);
          wasmDetail.wasm_bytes = buf.byteLength;
          mark('wasm_downloaded', { heap: heap() });
          const s1 = performance.now();
          wasmModule = await WebAssembly.compile(buf);
          wasmDetail.compile_ms = +(performance.now() - s1).toFixed(1);
          mark('wasm_compiled', { heap: heap() });
          const s2 = performance.now();
          instance = await WebAssembly.instantiate(wasmModule, imports);
          wasmDetail.instantiate_ms = +(performance.now() - s2).toFixed(1);
        } else if (c.mode === 'cachestorage') {
          // Cache Storage as the application-level cache: the first run stores the (decoded) response, later runs compile from it
          const cache = await caches.open('flengine6');
          let resp = await cache.match(c.wasm);
          wasmDetail.cache_hit = !!resp;
          if (!resp) {
            await cache.put(c.wasm, await fetch(url, { cache: 'no-store' }));
            wasmDetail.cache_put_ms = +(performance.now() - s).toFixed(1);
            mark('wasm_cached');
            resp = await cache.match(c.wasm);
          }
          const s1 = performance.now();
          const r = await WebAssembly.instantiateStreaming(resp, imports);
          wasmDetail.instantiate_streaming_from_cache_ms = +(performance.now() - s1).toFixed(1);
          instance = r.instance;
          wasmModule = r.module;
        } else {
          const r = await WebAssembly.instantiateStreaming(fetch(url, { cache: fc }), imports);
          instance = r.instance;
          wasmModule = r.module;
          wasmDetail.instantiate_streaming_ms = +(performance.now() - s).toFixed(1);
        }
        mark('wasm_instantiated', { heap: heap(), memory_bytes: instance.exports.memory.buffer.byteLength });
        done(instance, wasmModule);
      })().catch((e) => post({ type: 'error', phase: 'wasm', message: String(e && e.stack || e) }));
      return {};
    },
  };

  let error = null;
  try {
    await self.Module(opts);
  } catch (e) {
    error = String(e && e.stack || e);
  }
  const memBytes = instance ? instance.exports.memory.buffer.byteLength : null;
  mark('done', { heap: heap(), memory_bytes: memBytes });
  post({ type: 'done', exitCode, error, marks, wasmDetail, heap: heap(), memoryBytes: memBytes, stdoutLines: stdout.length, stderrLines: stderr.length, stdout, stderr });
}

self.onmessage = (ev) => {
  if (!ev.data || ev.data.cmd !== 'run') return;
  run(ev.data).catch((e) => post({ type: 'error', phase: 'run', message: String(e && e.stack || e) }));
};
