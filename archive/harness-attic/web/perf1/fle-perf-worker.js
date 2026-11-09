'use strict';
// perf1 (decision 0130, PROVISIONAL): a copy of ../fruity/fle-fruity-worker.js (decision 0128) that also records ABSOLUTE times (performance.timeOrigin + performance.now(): the page converts them to
// time since its own navigation start), can compile with WebAssembly.compileStreaming (c.streaming), and can let the HTTP cache work (c.cache). Everything else is the file of decision 0128.
// Web Worker that runs the FRUITY-on 6-module wasm (lift/harness/two/build_nmod_wasm.sh FRUITY=1 THREADS=1 JSPI=1 SJLJ=wasm, linked for the web by tools/wasm-relink-web.sh of the wf1 work dir;
// decision 0128, PROVISIONAL) the way lift/harness/two/run_node_fruity.cjs runs it under node: the same arguments (<FLEngine dll> <5 rebased module files joined with ':'>), the same environment
// (FLE_CALL_FRUITY=1, FLE_VFS_SEED, FLE_VFS_SEED_ROOT, FLE_REG_SEED, FLE_AUTH_CLAIMS, FLE_FRUITY_*), but the host files are fetched over HTTP and written to MEMFS before main().
// The window's drawing surface (the C side calls Module.fleSurfaceEvent synchronously, inside the engine's stack: kind 1 = the first non-background pixel, 2 = the stop) is shown two ways:
//   method A  this Worker owns the OffscreenCanvas of the page's canvas A (transferControlToOffscreen), putImageData, and reads it back (getImageData) at once;
//   method B  this Worker posts the RGBA bytes (ArrayBuffer, transferred) to the page, which draws them on its canvas B and reads that back.
// The SHA-256 of every read-back is made by the party that read it (crypto.subtle is asynchronous: the bytes are kept and hashed after the event).
// Messages to the page: {type:'mark',name,t}, {type:'stdout'|'stderr',line}, {type:'surface',...}, {type:'frame',...}, {type:'done',...}, {type:'error',message}.
// Message from the page: {cmd:'init', canvasA, ...} then {cmd:'run', glue, dataBase, env, jspiStack}.
const T0 = performance.now();
const ORIGIN = performance.timeOrigin;
const post = (m, tr) => self.postMessage(m, tr || []);
const marks = {};
const mark = (name, extra) => { const t = +(performance.now() - T0).toFixed(1); marks[name] = t; post(Object.assign({ type: 'mark', name, t, abs: ORIGIN + performance.now() }, extra)); return t; };

const MODULES = ['dsp_ippv2_x64.2000000.dll', 'Blend2D.4700000.dll', 'quickfontcache_x64.1d00000.dll', 'freetype_x64.4b00000.dll', 'ILTools_x64.6000000.dll'];
const FLE = 'FLEngine_x64.dll';
const SEED = ['seed/vfs-seed-fonts.tsv', 'seed/lift/out-inventory/extracted/fonts/ILCursors.ilfont', 'seed/lift/out-inventory/extracted/fonts/ILGlyphsEx.ilfont', 'seed/lift/out-fruity11/inputs/GM_CCs.txt',
  'seed/reg-seed-nsis.tsv', 'seed/reg-seed-app.tsv', 'seed/authenticode-claims.tsv'];

let canvasA = null, ctxA = null;
self.onmessage = (ev) => {
  const m = ev.data;
  if (m.cmd === 'init') { canvasA = m.canvasA; ctxA = canvasA.getContext('2d'); post({ type: 'ready' }); return; }
  if (m.cmd === 'run') run(m).catch((e) => post({ type: 'error', phase: 'run', message: String((e && e.stack) || e) }));
  // a message of the page while the engine runs: this handler runs only when the Worker's event loop turns (G3: the engine's JSPI switches are microtasks)
  if (m.cmd === 'ping') {
    pings.push({ seq: m.seq, sentPageMs: m.sentPageMs, receivedWorkerMs: +(performance.now() - T0).toFixed(1), workerTimeOrigin: performance.timeOrigin, engineStage: stage });
    post({ type: 'pong', seq: m.seq, sentPageMs: m.sentPageMs, workerMs: +(performance.now() - T0).toFixed(2), stage });
  }
  // G3 (decision 0128): an input event of the page while the engine is idle (suspended in fle_idle_wait). Only messages of the allow-list of harness/fruity_idle.c are accepted by the C side.
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
// every JS error that reaches the Worker's global scope is reported to the page (decision 0128: the first Chrome run left nothing because only `done` was reported)
self.addEventListener('error', (e) => post({ type: 'jserror', message: String(e.message), filename: e.filename, lineno: e.lineno }));
self.addEventListener('unhandledrejection', (e) => post({ type: 'jserror', message: 'unhandledrejection: ' + String((e.reason && e.reason.stack) || e.reason).slice(0, 600) }));
self.wf1State = () => ({ stage, ticks, pings: pings.length, finished });

async function sha256hex(u8) {
  const d = await crypto.subtle.digest('SHA-256', u8);
  return [...new Uint8Array(d)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

async function run(c) {
  const root = c.root || '/data';
  const stdout = [], stderr = [];
  let exitCode = null, instance = null, memory = null;
  const grows = [];
  let prevIdle = null;
  const events = [];            // the surface events: {kind, lid, w, h, tMs, rgba: Uint8Array copy of what the canvas read-back gave (A) , ...}
  const t0 = performance.now();

  // memory.grow bookkeeping: every grow() of every WebAssembly.Memory of this Worker (the glue's wasmMemory.grow through emscripten_resize_heap)
  const origGrow = WebAssembly.Memory.prototype.grow;
  WebAssembly.Memory.prototype.grow = function (n) { const before = this.buffer.byteLength; const r = origGrow.call(this, n); grows.push({ t: +(performance.now() - T0).toFixed(1), pages: n, from: before, to: this.buffer.byteLength }); return r; };

  tickTimer = setInterval(() => { ticks++; }, 10);   // counts the macrotasks the Worker's event loop runs (G3 evidence)
  importScripts(c.glue);
  mark('glue_loaded');
  const names = [FLE, ...MODULES, ...SEED];
  const f0 = performance.now();
  const bufs = await Promise.all(names.map(async (n) => {
    const r = await fetch(c.dataBase + n, { cache: c.cache ? 'default' : 'no-store' });
    if (!r.ok) throw new Error(n + ': HTTP ' + r.status);
    return new Uint8Array(await r.arrayBuffer());
  }));
  mark('data_fetched', { files: names.length, bytes: bufs.reduce((s, b) => s + b.length, 0), ms: +(performance.now() - f0).toFixed(1) });

  const env = Object.assign({
    FLE_CALL_FRUITY: '1',
    FLE_VFS_SEED: root + '/seed/vfs-seed-fonts.tsv',
    FLE_VFS_SEED_ROOT: root + '/seed',
    FLE_REG_SEED: root + '/seed/reg-seed-nsis.tsv:' + root + '/seed/reg-seed-app.tsv',
    FLE_AUTH_CLAIMS: root + '/seed/authenticode-claims.tsv',
    FLE_FRUITY_MAX_CALLS: '400000000',
    FLE_FRUITY_SECONDS: '3000',
  }, c.env || {});

  const opts = {
    arguments: [root + '/' + FLE, MODULES.map((m) => root + '/' + m).join(':')],
    print: (line) => { stdout.push(line); post({ type: 'stdout', line }); },
    printErr: (line) => { stderr.push(line); post({ type: 'stderr', line }); },
    // the WEB glue never reads Module.quit (only the node branch does): the exit status comes through onExit (called by _proc_exit), and finish() also runs after Module() returns or rejects
    onExit: (code) => {
      post({ type: 'quit', code, t: +(performance.now() - T0).toFixed(1) });
      if (exitCode === null) exitCode = code;
      mark('exit', { code });
      // exit/abort raised on a guest thread other than the first leaves the first thread suspended for ever: Module() never settles (decision 0124's thread lib says so), so finish() is also started here
      setTimeout(() => { if (!finished) { finished = true; finish(null).catch((x) => post({ type: 'error', phase: 'finish', message: String(x) })); } }, 0);
    },
    fleProgress: (n) => post({ type: 'progress', entries: n, t: +(performance.now() - T0).toFixed(1) }),
    onAbort: (what) => { post({ type: 'stderr', line: 'onAbort: ' + what }); },
    onRuntimeInitialized: () => { stage = 'engine-running'; mark('main_start'); },
    fleIdle: (n, entries) => { stage = 'idle'; post({ type: 'idle', n, entries, t: +(performance.now() - T0).toFixed(2) }); },
    fleThreadFailed: (idx, e, msg) => { post({ type: 'stderr', line: '[js] ' + msg }); },
    preRun: [() => {
      const FS = opts.FS;
      names.forEach((n, i) => { const g = root + '/' + n; FS.mkdirTree(g.slice(0, g.lastIndexOf('/'))); FS.writeFile(g, bufs[i], { canOwn: true }); });
      for (const k of Object.keys(env)) opts.ENV[k] = env[k];
      mark('memfs_ready');
    }],
    instantiateWasm(imports, done) {
      (async () => {
        const s = performance.now();
        mark('wasm_start');
        let mod;
        if (c.cacheapi === 'gz') {
          // Cache API copy of the GZIP bytes (the file <wasm>.gz served as an opaque file, no Content-Encoding): the cache holds the encoded size (decision 0130: a 500 MB wasm cannot be put decoded, a 60 MB gzip can);
          // every start reads it from Cache Storage, decodes it with DecompressionStream('gzip') and hands the stream to compileStreaming.
          const gzUrl = c.wasm + '.gz';
          const cache = await caches.open('perf1-wasm-gz');
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
        } else if (c.cacheapi) {
          // a Cache API copy of the (compressed-on-the-wire, decoded-in-the-cache) wasm: the first start fetches and puts it, the next start reads it from Cache Storage and compiles it with compileStreaming
          // (V8's code cache for a streamed module is keyed by the response's URL and works only when the embedder writes it; whether Chrome 156 does for a Cache Storage response is what the second start measures).
          const cache = await caches.open('perf1-wasm');
          let resp = await cache.match(c.wasm);
          const hit = !!resp;
          if (!resp) {
            const net = await fetch(c.wasm, { cache: 'no-store' });
            await cache.put(c.wasm, net.clone());
            mark('wasm_put_in_cache_api', { ms: +(performance.now() - s).toFixed(1) });
            resp = await cache.match(c.wasm);
          }
          const s2 = performance.now();
          mod = await WebAssembly.compileStreaming(resp);
          mark('wasm_downloaded', { cacheapi: true, hit, ms: +(performance.now() - s).toFixed(1) });
          mark('wasm_compiled', { cacheapi: true, hit, ms: +(performance.now() - s2).toFixed(1) });
        } else if (c.streaming) {
          // compileStreaming: the download and the compile overlap (the V8 streaming decoder); the Response is consumed here, 'wasm_downloaded' is when the compile promise settles
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
        instance = await WebAssembly.instantiate(mod, imports);
        eng = { instance, opts };
        memory = instance.exports.memory;
        mark('wasm_instantiated', { memory_bytes: memory.buffer.byteLength });
        done(instance, mod);
      })().catch((e) => post({ type: 'error', phase: 'wasm', message: String((e && e.stack) || e) }));
      return {};
    },
    // the C side (harness/fruity_surface.c) calls this synchronously; the pixels are BGRX (B, G, R, X = 0), top-down, `stride` bytes per row
    fleSurfaceEvent(kind, lid, pixels, w, h, stride) {
      const tEvent = performance.now() - T0;
      const ev = { kind, lid, w, h, stride, pixels, t: +tEvent.toFixed(1), abs: ORIGIN + performance.now(), memPages: memory.buffer.byteLength / 65536 };
      if (kind === 3) stage = 'idle';
      ev.main = lid === mainLidOf(instance, memory);
      if (ev.main) {
        const src = new Uint32Array(memory.buffer, pixels, (stride * h) >> 2);
        const rgba = new Uint32Array(w * h);
        const sw = stride >> 2;
        for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { const v = src[y * sw + x]; rgba[y * w + x] = 0xff000000 | ((v & 0xff) << 16) | (v & 0xff00) | ((v >>> 16) & 0xff); }
        ev.rgbaCopy = rgba;
        if (kind === 3) {                              // idle: how many pixels differ from the previous idle surface, and where (the engine's answer to an input event)
          if (prevIdle && prevIdle.length === rgba.length) {
            let n = 0, x0 = w, y0 = h, x1 = -1, y1 = -1;
            for (let i = 0; i < rgba.length; i++) if (rgba[i] !== prevIdle[i]) { n++; const x = i % w, y = (i / w) | 0; if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
            ev.diffPrev = { pixels: n, bbox: n ? [x0, y0, x1 + 1, y1 + 1] : null };
          }
          prevIdle = rgba;
        }
        const t1 = performance.now();
        if (ctxA) {                                   // method A
          if (canvasA.width !== w || canvasA.height !== h) { canvasA.width = w; canvasA.height = h; }
          ctxA.putImageData(new ImageData(new Uint8ClampedArray(rgba.buffer.slice(0)), w, h), 0, 0);
          ev.aReadback = new Uint8Array(ctxA.getImageData(0, 0, w, h).data.buffer);
          ev.aMs = +(performance.now() - t1).toFixed(2);
        }
        const copy = rgba.buffer.slice(0);              // method B: a transferred copy
        post({ type: 'frame', kind, lid, main: true, w, h, data: copy, t: ev.t, abs: ev.abs }, [copy]);
      }
      events.push(ev);
    },
  };

  async function finish(error) {
    const memBytes = memory ? memory.buffer.byteLength : null;
    const out = [];
    for (const ev of events) {
      const r = { kind: ev.kind, lid: ev.lid, main: ev.main, w: ev.w, h: ev.h, stride: ev.stride, pixels: ev.pixels, t: ev.t, abs: ev.abs, memPages: ev.memPages };
      if (ev.diffPrev) r.diffPrev = ev.diffPrev;
      if (ev.rgbaCopy) r.rgbaSha256 = await sha256hex(new Uint8Array(ev.rgbaCopy.buffer));
      if (ev.aReadback) { r.canvasA_readback_sha256 = await sha256hex(ev.aReadback); r.aMs = ev.aMs; }
      out.push(r);
    }
    mark('done');
    stage = 'finished';
    clearInterval(tickTimer);
    post({ type: 'done', timeOrigin: ORIGIN, exitCode, error, marks, grows, events: out, workerTicksAtDone: ticks, pings, memoryBytes: memBytes, stdout, stderr, stdoutLines: stdout.length, ms: +(performance.now() - t0).toFixed(1) });
  }

  try { await self.Module(opts); post({ type: 'jserror', message: 'Module() resolved (exitCode ' + exitCode + ')' }); if (!finished) { finished = true; await finish(null); } } catch (e) {
    // exit() inside the first thread unwinds main with ExitStatus (quit has run); anything else is a trap
    post({ type: 'jserror', message: 'Module() rejected: ' + String((e && e.stack) || e).slice(0, 800) });
    if (!(e && e.name === 'ExitStatus') && !finished) { finished = true; post({ type: 'stderr', line: 'trap/exception: ' + String((e && e.stack) || e).slice(0, 600) }); finish(String((e && e.stack) || e)).catch(() => {}); }
  }
}
