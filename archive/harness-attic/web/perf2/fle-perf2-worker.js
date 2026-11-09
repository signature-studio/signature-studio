'use strict';
// perf2 (decision 0138, PROVISIONAL): a copy of ../final0129/fle-final-worker.js (decision 0134) for long runs. Differences: (1) Module.fleSample(wall, virtual, frames) is called by perf2_probe.c at every sample (a message {type:'sample'} with the linear memory, the number of memory.grow calls, the Worker's 10 ms timer ticks and the JS heap);
// (2) Module.fleSurfaceEvent keeps no RGBA copy (an 8 MB array per sample would grow without end): it copies the surface, draws lid 13 on canvas A, posts the copy to the page (transferred) and hashes it asynchronously; (3) no snapshot of events is kept beyond a small record.
// Original header of final0129:
// final0129 (decision 0134, PROVISIONAL): the Worker of the FRUITY-on wasm in decision 0129's FINAL configuration (valid up to wall 16, kernel32!RaiseException 0x0EEDFADE; SEH is not modelled). A copy of ../perf1/fle-perf-worker.js
// (decision 0130; itself a copy of ../fruity/fle-fruity-worker.js, decision 0128) with these differences:
//   - the VFS seed does not come as 7 loose files: ONE pack file (data/seedpack.bin, tools: pack_seed.py) holds the seed list (vfs-seed-idle1.tsv), its 218 host files, the registry seeds and the claims. The Worker fetches it, parses its
//     header and writes every file to MEMFS (FS.writeFile, canOwn: the MEMFS node keeps a view into the pack's ArrayBuffer, no copy) BEFORE main(); the C side (vfs_seed.h) is unchanged: it fopen()s the same relative paths.
//   - the environment of decision 0129 (FLE_VFS_SEED = vfs-seed-idle1.tsv, FLE_FRUITY_NO_IDLE_STOP=1, FLE_FRUITY_SECONDS, FLE_FRUITY_MAX_CALLS).
//   - marks for the four times of the work: pack_fetched (the pack's bytes are in hand), wasm_instantiated (the wasm is ready), the first non-background pixel (frame kind 1), the stop (frame kind 2); every resource also has a
//     fetched:<name> mark. c.par = 1 starts the wasm download+compile at the same time as the data fetch (default: after it, as in perf1).
//   - c.cacheapi = 'gz' now covers wasm, pack and the six DLLs: the file <name>.gz is served as an opaque file, kept in the Cache API, and decoded with DecompressionStream('gzip') at every start (decision 0130: a 526 MB wasm cannot be
//     put decoded, a 60 MB gzip can).
//   - heap probes: when the wasm exports fle_heap_probe (HEAPPROBE=1 build), the linear memory in use (mallinfo), the heap top and the memory size are recorded at the stdout lines of the seeding and at the surface events.
// Messages to the page: {type:'mark',name,t,abs}, {type:'stdout'|'stderr',line}, {type:'frame',...}, {type:'done',...}, {type:'error',message}. Message from the page: {cmd:'init', canvasA} then {cmd:'run', glue, wasm, dataBase, env, ...}.
const T0 = performance.now();
const ORIGIN = performance.timeOrigin;
const post = (m, tr) => self.postMessage(m, tr || []);
const marks = {};
const mark = (name, extra) => { const t = +(performance.now() - T0).toFixed(1); marks[name] = t; post(Object.assign({ type: 'mark', name, t, abs: ORIGIN + performance.now() }, extra)); return t; };

const MODULES = ['dsp_ippv2_x64.2000000.dll', 'Blend2D.4700000.dll', 'quickfontcache_x64.1d00000.dll', 'freetype_x64.4b00000.dll', 'ILTools_x64.6000000.dll'];
const FLE = 'FLEngine_x64.dll';

let canvasA = null, ctxA = null;
self.onmessage = (ev) => {
  const m = ev.data;
  if (m.cmd === 'init') { canvasA = m.canvasA; ctxA = canvasA.getContext('2d'); post({ type: 'ready' }); return; }
  if (m.cmd === 'run') run(m).catch((e) => post({ type: 'error', phase: 'run', message: String((e && e.stack) || e) }));
  if (m.cmd === 'ping') {
    pings.push({ seq: m.seq, sentPageMs: m.sentPageMs, receivedWorkerMs: +(performance.now() - T0).toFixed(1), workerTimeOrigin: performance.timeOrigin, engineStage: stage });
    post({ type: 'pong', seq: m.seq, sentPageMs: m.sentPageMs, workerMs: +(performance.now() - T0).toFixed(2), stage });
  }
  if (m.cmd === 'input') {
    const t = +(performance.now() - T0).toFixed(2);
    if (!eng || !eng.opts.fleIdleWaiting || !eng.opts.fleIdleWaiting()) { post({ type: 'inputAck', seq: m.seq, r: -9, why: 'the engine is not waiting in idle', t }); return; }
    const r = eng.instance.exports.fle_host_post_input(m.lid === 'main' ? mainLidOf(eng.instance, eng.instance.exports.memory) : m.lid, m.msg, m.wp >>> 0, m.lp >>> 0);
    post({ type: 'inputAck', seq: m.seq, r, t, sentPageMs: m.sentPageMs });
    if (r === 1) eng.opts.fleIdleDeliver(1);
  }
  if (m.cmd === 'end') { if (eng && eng.opts.fleIdleWaiting && eng.opts.fleIdleWaiting()) { post({ type: 'inputAck', seq: -1, r: 0, why: 'end', t: +(performance.now() - T0).toFixed(2) }); eng.opts.fleIdleDeliver(0); } }
};
// ---- main-window lookup (decision 0150) ----
// The MAIN window is found by what it is, not by its lid: the window of class TFruityLoopsMainForm in the engine's window list (fle_av_poll, lift/runtime/fle_appview.h: header 68 bytes, entries of 144 bytes,
// lid at +0, class name at +48), when the wasm exports it (builds since decision 0141). The wasm builds this tool was written for (decisions 0128-0138) export nothing of the kind: their main form was lid 13 by
// measurement (decision 0126), which is true for those builds only. Returns the lid, or null when the engine's list has no such window.
const MAIN_FORM_CLASS = 'TFruityLoopsMainForm', MAIN_LID_OF_THE_OLD_BUILDS = 13;
function mainLidOf(instance, memory) {
  const ex = instance && instance.exports;
  if (!ex || typeof ex.fle_av_poll !== 'function' || !memory) return MAIN_LID_OF_THE_OLD_BUILDS;
  const p = ex.fle_av_poll(), dv = new DataView(memory.buffer, p), u8 = new Uint8Array(memory.buffer, p);
  if (dv.getUint32(0, true) !== 0x50414c46 || dv.getUint32(8, true) !== 144) return null;
  const n = Math.min(dv.getUint32(44, true), dv.getUint32(12, true));
  for (let i = 0; i < n; i++) {
    const b = 68 + i * 144;
    let e = b + 48;
    while (e < b + 80 && u8[e] !== 0) e++;
    if (String.fromCharCode(...u8.subarray(b + 48, e)) === MAIN_FORM_CLASS) return dv.getUint32(b, true);
  }
  return null;
}
// ---- end of main-window lookup ----
let eng = null;
const pings = [];
let stage = 'before-run', ticks = 0, tickTimer = null, finished = false;
self.addEventListener('error', (e) => post({ type: 'jserror', message: String(e.message), filename: e.filename, lineno: e.lineno }));
self.addEventListener('unhandledrejection', (e) => post({ type: 'jserror', message: 'unhandledrejection: ' + String((e.reason && e.reason.stack) || e.reason).slice(0, 600) }));
self.wf1State = () => ({ stage, ticks, pings: pings.length, finished });

async function sha256hex(u8) {
  const d = await crypto.subtle.digest('SHA-256', u8);
  return [...new Uint8Array(d)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

// the bytes of one resource: over the network (Chrome decodes Content-Encoding itself), or, with c.cacheapi === 'gz', from the Cache API copy of <name>.gz (put there by the first start)
async function getBytes(c, url, info) {
  if (c.cacheapi !== 'gz') {
    const r = await fetch(url, { cache: c.cache ? 'default' : 'no-store' });
    if (!r.ok) throw new Error(url + ': HTTP ' + r.status);
    return new Uint8Array(await r.arrayBuffer());
  }
  const gzUrl = url + '.gz';
  const cache = await caches.open('perf2-gz');
  let resp = await cache.match(gzUrl);
  const hit = !!resp;
  if (!resp) {
    const net = await fetch(gzUrl, { cache: 'no-store' });
    if (!net.ok) throw new Error(gzUrl + ': HTTP ' + net.status);
    await cache.put(gzUrl, net.clone());
    resp = await cache.match(gzUrl);
  }
  info.hit = hit;
  const dec = new Response(resp.body.pipeThrough(new DecompressionStream('gzip')));
  return new Uint8Array(await dec.arrayBuffer());
}

async function wasmModule(c, imports_unused) {
  const s = performance.now();
  mark('wasm_start');
  let mod;
  if (c.cacheapi === 'gz') {
    const gzUrl = c.wasm + '.gz';
    const cache = await caches.open('perf2-gz');
    let resp = await cache.match(gzUrl);
    const hit = !!resp;
    if (!resp) {
      const net = await fetch(gzUrl, { cache: 'no-store' });
      await cache.put(gzUrl, net.clone());
      mark('wasm_put_in_cache_api', { ms: +(performance.now() - s).toFixed(1) });
      resp = await cache.match(gzUrl);
    }
    const s2 = performance.now();
    const decoded = new Response(resp.body.pipeThrough(new DecompressionStream('gzip')), { headers: { 'Content-Type': 'application/wasm' } });
    mod = await WebAssembly.compileStreaming(decoded);
    mark('wasm_downloaded', { cacheapi: 'gz', hit, ms: +(performance.now() - s).toFixed(1) });
    mark('wasm_compiled', { cacheapi: 'gz', hit, ms: +(performance.now() - s2).toFixed(1) });
  } else if (c.streaming) {
    const resp = await fetch(c.wasm, { cache: c.cache ? 'default' : 'no-store' });
    mod = await WebAssembly.compileStreaming(resp);
    mark('wasm_downloaded', { streaming: true, ms: +(performance.now() - s).toFixed(1) });
    mark('wasm_compiled', { streaming: true, ms: +(performance.now() - s).toFixed(1) });
  } else {
    const buf = await (await fetch(c.wasm, { cache: c.cache ? 'default' : 'no-store' })).arrayBuffer();
    mark('wasm_downloaded', { bytes: buf.byteLength, ms: +(performance.now() - s).toFixed(1) });
    const s1 = performance.now();
    mod = await WebAssembly.compile(buf);
    mark('wasm_compiled', { ms: +(performance.now() - s1).toFixed(1) });
  }
  return mod;
}

function parsePack(buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const magic = String.fromCharCode(...buf.subarray(0, 8));
  if (magic !== 'FLSEEDP1') throw new Error('seed pack: bad magic ' + magic);
  const hl = dv.getUint32(8, true);
  const header = JSON.parse(new TextDecoder().decode(buf.subarray(12, 12 + hl)));
  return { header, dataStart: 12 + hl };
}

async function run(c) {
  const root = c.root || '/data';
  const stdout = [], stderr = [];
  let exitCode = null, instance = null, memory = null;
  const grows = [];
  const heap = [];
    const events = [];
  const t0 = performance.now();
  const probe = (tag) => {
    const ex = instance && instance.exports;
    if (!ex || !ex.fle_heap_probe) return;
    const r = { tag, t: +(performance.now() - T0).toFixed(1), inUse: ex.fle_heap_probe(0), free: ex.fle_heap_probe(1), arena: ex.fle_heap_probe(2), top: ex.fle_heap_probe(3), memSize: ex.fle_heap_probe(4) };
    if (performance.memory) r.jsHeapUsed = performance.memory.usedJSHeapSize;
    heap.push(r);
  };

  const origGrow = WebAssembly.Memory.prototype.grow;
  WebAssembly.Memory.prototype.grow = function (n) { const before = this.buffer.byteLength; const r = origGrow.call(this, n); grows.push({ t: +(performance.now() - T0).toFixed(1), pages: n, from: before, to: this.buffer.byteLength }); return r; };

  tickTimer = setInterval(() => { ticks++; }, 10);
  importScripts(c.glue);
  mark('glue_loaded');

  // the six DLLs and the seed pack: fetched together; each has its own mark when its bytes are in hand
  const dllNames = [FLE, ...MODULES];
  const packName = c.pack || 'seedpack.bin';
  const infos = {};
  const f0 = performance.now();
  const fetchOne = async (n) => {
    infos[n] = {};
    const b = await getBytes(c, c.dataBase + n, infos[n]);
    mark('fetched:' + n, { bytes: b.length, ms: +(performance.now() - f0).toFixed(1), hit: infos[n].hit });
    return b;
  };
  let wasmP = null;
  if (c.par) wasmP = wasmModule(c);                 // the wasm download+compile overlaps with the data fetch (c.par = 1)
  // c.seedmode: 'pack' (default: the seed pack is fetched and written to MEMFS here), 'embed' (the seed files were linked into the wasm by emcc --embed-file) or 'preload' (emcc --preload-file: the glue fetches flengine6.data itself)
  const seedmode = c.seedmode || 'pack';
  const [packBytes, ...dllBytes] = await Promise.all([seedmode === 'pack' ? fetchOne(packName) : Promise.resolve(new Uint8Array(0)), ...dllNames.map(fetchOne)]);
  mark('data_fetched', { files: (seedmode === 'pack' ? 1 : 0) + dllNames.length, bytes: packBytes.length + dllBytes.reduce((s, b) => s + b.length, 0), pack_bytes: packBytes.length, ms: +(performance.now() - f0).toFixed(1), seedmode });

  const env = Object.assign({
    FLE_CALL_FRUITY: '1',
    FLE_VFS_SEED: root + '/seed/vfs-seed-idle1.tsv',
    FLE_VFS_SEED_ROOT: root + '/seed',
    FLE_REG_SEED: root + '/seed/reg-seed-nsis.tsv:' + root + '/seed/reg-seed-app.tsv',
    FLE_AUTH_CLAIMS: root + '/seed/authenticode-claims.tsv',
    FLE_FRUITY_NO_IDLE_STOP: '1',
    FLE_FRUITY_MAX_CALLS: '4000000000',
    FLE_FRUITY_SECONDS: '3000',
  }, c.env || {});

  const opts = {
    locateFile: (p) => new URL(p, c.glue).href,                       // flengine6.data of a --preload-file link is next to the glue
    monitorRunDependencies: (n) => { if (seedmode !== 'pack') mark('run_dependencies:' + n); },
    arguments: [root + '/' + FLE, MODULES.map((m) => root + '/' + m).join(':')],
    print: (line) => {
      stdout.push(line); post({ type: 'stdout', line });
      if (line.startsWith('vfs seed: list ')) probe('vfs-seed-start');
      else if (line.startsWith('vfs seed: done')) probe('vfs-seed-done');
      else if (line.startsWith('fruity: calling CreateFruityInstance')) probe('before-CreateFruityInstance');
    },
    printErr: (line) => { stderr.push(line); post({ type: 'stderr', line }); },
    onExit: (code) => {
      post({ type: 'quit', code, t: +(performance.now() - T0).toFixed(1) });
      if (exitCode === null) exitCode = code;
      mark('exit', { code });
      setTimeout(() => { if (!finished) { finished = true; finish(null).catch((x) => post({ type: 'error', phase: 'finish', message: String(x) })); } }, 0);
    },
    fleProgress: (n) => post({ type: 'progress', entries: n, t: +(performance.now() - T0).toFixed(1) }),
    onAbort: (what) => { post({ type: 'stderr', line: 'onAbort: ' + what }); },
    onRuntimeInitialized: () => { stage = 'engine-running'; mark('main_start'); probe('main-start'); },
    fleIdle: (n, entries) => { stage = 'idle'; post({ type: 'idle', n, entries, t: +(performance.now() - T0).toFixed(2) }); },
    fleThreadFailed: (idx, e, msg) => { post({ type: 'stderr', line: '[js] ' + msg }); },
    preRun: [() => {
      const FS = opts.FS;
      const m0 = memory ? memory.buffer.byteLength : null;
      const w0 = performance.now();
      const jsBefore = performance.memory ? performance.memory.usedJSHeapSize : null;
      dllNames.forEach((n, i) => { const g = root + '/' + n; FS.mkdirTree(g.slice(0, g.lastIndexOf('/'))); FS.writeFile(g, dllBytes[i], { canOwn: true }); });
      const w1 = performance.now();
      const { header, dataStart } = seedmode === 'pack' ? parsePack(packBytes) : { header: { files: [] }, dataStart: 0 };
      let nfiles = 0, nbytes = 0;
      for (const f of header.files) {
        const g = root + '/' + f.path;
        FS.mkdirTree(g.slice(0, g.lastIndexOf('/')));
        FS.writeFile(g, packBytes.subarray(dataStart + f.offset, dataStart + f.offset + f.size), { canOwn: true });
        nfiles++; nbytes += f.size;
      }
      const w2 = performance.now();
      for (const k of Object.keys(env)) opts.ENV[k] = env[k];
      mark('memfs_ready', { dll_write_ms: +(w1 - w0).toFixed(1), pack_files: nfiles, pack_data_bytes: nbytes, pack_write_ms: +(w2 - w1).toFixed(1), memory_before: m0, memory_after: memory ? memory.buffer.byteLength : null,
        jsHeapUsedBefore: jsBefore, jsHeapUsedAfter: performance.memory ? performance.memory.usedJSHeapSize : null });
    }],
    instantiateWasm(imports, done) {
      (async () => {
        const mod = await (wasmP || wasmModule(c));
        instance = await WebAssembly.instantiate(mod, imports);
        eng = { instance, opts };
        memory = instance.exports.memory;
        mark('wasm_instantiated', { memory_bytes: memory.buffer.byteLength });
        done(instance, mod);
      })().catch((e) => post({ type: 'error', phase: 'wasm', message: String((e && e.stack) || e) }));
      return {};
    },
    fleSample(wall, virt, frames) {
      post({ type: 'sample', wall, virt, frames, t: +(performance.now() - T0).toFixed(1), abs: ORIGIN + performance.now(), memBytes: memory ? memory.buffer.byteLength : null, grows: grows.length, ticks, jsHeap: performance.memory ? performance.memory.usedJSHeapSize : null });
    },
    fleSurfaceEvent(kind, lid, pixels, w, h, stride) {
      const tEvent = performance.now() - T0;
      const sw = stride >> 2;
      const src = new Uint32Array(memory.buffer, pixels, sw * h);
      const rgba = new Uint32Array(w * h);
      for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { const v = src[y * sw + x]; rgba[y * w + x] = 0xff000000 | ((v & 0xff) << 16) | (v & 0xff00) | ((v >>> 16) & 0xff); }
      const ev = { kind, lid, w, h, stride, pixels, t: +tEvent.toFixed(1), abs: ORIGIN + performance.now(), memPages: memory.buffer.byteLength / 65536 };
      ev.main = lid === mainLidOf(instance, memory);
      if (ev.main && (kind === 1 || kind === 2)) probe(kind === 1 ? 'first-pixel' : 'stop');
      if (ev.main && ctxA) {
        const t1 = performance.now();
        if (canvasA.width !== w || canvasA.height !== h) { canvasA.width = w; canvasA.height = h; }
        ctxA.putImageData(new ImageData(new Uint8ClampedArray(rgba.buffer.slice(0)), w, h), 0, 0);
        ev.aMs = +(performance.now() - t1).toFixed(2);
        if (kind === 1 || kind === 2) ev.aReadback = new Uint8Array(ctxA.getImageData(0, 0, w, h).data.buffer);
      }
      const copy = rgba.buffer.slice(0);
      post({ type: 'frame', kind, lid, main: ev.main, w, h, data: copy, t: ev.t, abs: ev.abs }, [copy]);
      ev.sha = sha256hex(new Uint8Array(rgba.buffer));      // a promise: the hash of the copy, computed after this call returns
      events.push(ev);
      if (events.length > 400) events.splice(0, events.length - 400);
    },
  };

  async function finish(error) {
    const memBytes = memory ? memory.buffer.byteLength : null;
    probe('end');
    const out = [];
    for (const ev of events) {
      const r = { kind: ev.kind, lid: ev.lid, main: ev.main, w: ev.w, h: ev.h, stride: ev.stride, pixels: ev.pixels, t: ev.t, abs: ev.abs, memPages: ev.memPages };
      if (ev.sha) r.rgbaSha256 = await ev.sha;
      if (ev.aReadback) { r.canvasA_readback_sha256 = await sha256hex(ev.aReadback); r.aMs = ev.aMs; }
      out.push(r);
    }
    mark('done');
    stage = 'finished';
    clearInterval(tickTimer);
    post({ type: 'done', timeOrigin: ORIGIN, exitCode, error, marks, grows, heap, events: out, workerTicksAtDone: ticks, pings, memoryBytes: memBytes, stdout, stderr, stdoutLines: stdout.length, ms: +(performance.now() - t0).toFixed(1) });
  }

  try { await self.Module(opts); post({ type: 'jserror', message: 'Module() resolved (exitCode ' + exitCode + ')' }); if (!finished) { finished = true; await finish(null); } } catch (e) {
    post({ type: 'jserror', message: 'Module() rejected: ' + String((e && e.stack) || e).slice(0, 800) });
    if (!(e && e.name === 'ExitStatus') && !finished) { finished = true; post({ type: 'stderr', line: 'trap/exception: ' + String((e && e.stack) || e).slice(0, 600) }); finish(String((e && e.stack) || e)).catch(() => {}); }
  }
}
