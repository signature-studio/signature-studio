'use strict';
// Web Worker of the G3 round trip (decision 0136, PROVISIONAL; derived from web/fruity/fle-fruity-worker.js of decision 0128). It runs the FRUITY-on 6-module wasm (build_nmod_wasm.sh FRUITY=1 THREADS=1 JSPI=1
// SJLJ=wasm, linked for the web by web/fruity/link-web.sh) and, while the engine is idle (suspended in the async import fle_idle_wait, harness/web/fruity/fle_idle_lib.js), turns the page's messages into calls of the
// engine's exports: raw mouse and key events (fle_g3_mouse, fle_g3_key) and the virtual clock. The page makes no Windows message: runtime/fle_input.c does, when the owning thread retrieves the event.
// Clock modes (set by the page, `mode`):
//   fixed  the virtual clock moves only when the page says `advance` (ms); the engine runs to its next idle after every command. This is the mode whose results are compared with the native script run.
//   wall   at an idle the Worker waits with setTimeout for the earliest deadline of the engine (the `next` the idle hook was given) and then moves the clock to exactly that deadline (so a deadline is delivered at its
//          own virtual time); an input event moves the clock by the wall time that passed since the idle began (rate = virtual ms per wall ms, default 1), never past the deadline. S-idle-clock-wall.
// Messages to the page: {type:'mark'|'stdout'|'stderr'|'idle'|'surface'|'snapDone'|'idleWake'|'ack'|'pong'|'done'|'error'|'quit'|'jserror'|'progress', ...}.
// Messages from the page: {cmd:'init'} {cmd:'run', glue, wasm, dataBase, root, env} {cmd:'mode', mode, rate} {cmd:'mouse', kind, x, y, data} {cmd:'key', kind, vk, scan, ext} {cmd:'advance', ms} {cmd:'snapshot'} {cmd:'timers'}
//   {cmd:'ping', seq, sentPageMs} {cmd:'end'}
const T0 = performance.now();
const post = (m, tr) => self.postMessage(m, tr || []);
const marks = {};
const mark = (name, extra) => { const t = +(performance.now() - T0).toFixed(1); marks[name] = t; post(Object.assign({ type: 'mark', name, t }, extra)); return t; };

const MODULES = ['dsp_ippv2_x64.2000000.dll', 'Blend2D.4700000.dll', 'quickfontcache_x64.1d00000.dll', 'freetype_x64.4b00000.dll', 'ILTools_x64.6000000.dll'];
const FLE = 'FLEngine_x64.dll';
const SEED = ['seed/vfs-seed-fonts.tsv', 'seed/lift/out-inventory/extracted/fonts/ILCursors.ilfont', 'seed/lift/out-inventory/extracted/fonts/ILGlyphsEx.ilfont', 'seed/lift/out-fruity11/inputs/GM_CCs.txt',
  'seed/reg-seed-nsis.tsv', 'seed/reg-seed-app.tsv', 'seed/authenticode-claims.tsv'];

let eng = null, mode = 'fixed', rate = 1;
let idle = null;            // {n, now, next, wall}: the engine is suspended in its idle
let timer = null;
let advanceTarget = 0;      // ticks: a pending `advance` command (fixed mode)
let idleCount = 0, ticks = 0, stage = 'before-run', finished = false, tickTimer = null;
const pings = [], idleLog = [];

function deliver(code) {
  if (timer) { clearTimeout(timer); timer = null; }
  const was = idle;
  idle = null;
  if (eng && eng.opts.fleIdleDeliver) eng.opts.fleIdleDeliver(code);
  return was;
}
function exportsOk() { return eng && eng.instance && eng.instance.exports.fle_g3_mouse; }
// the idle decides what to do next: a pending advance (fixed mode) or a timer to the next deadline (wall mode); otherwise it waits for a message
function onIdle() {
  if (!idle) return;
  if (advanceTarget) {
    if (idle.now >= advanceTarget) { advanceTarget = 0; post({ type: 'advanceDone', n: idle.n, now: idle.now }); }
    else { deliver(advanceTarget); return; }
  }
  if (mode === 'wall' && idle.next > 0) {
    const waitMs = Math.max(0, (idle.next - idle.now) / 10000 / rate);
    const n = idle.n;
    timer = setTimeout(() => {
      timer = null;
      if (!idle || idle.n !== n) return;
      const wallNow = performance.now();
      post({ type: 'idleWake', n, waitMs: +waitMs.toFixed(3), lateMs: +(wallNow - idle.wall - waitMs).toFixed(3), now: idle.next });
      deliver(idle.next);
    }, waitMs);
  }
}
function moveClockForWall() {
  if (mode !== 'wall' || !idle) return;
  let t = idle.now + (performance.now() - idle.wall) * 10000 * rate;
  if (idle.next > 0 && t > idle.next) t = idle.next;
  if (t > idle.now) eng.instance.exports.fle_g3_clock_to(t);
}

self.onmessage = (ev) => {
  const m = ev.data;
  if (m.cmd === 'init') { post({ type: 'ready' }); return; }
  if (m.cmd === 'run') { run(m).catch((e) => post({ type: 'error', phase: 'run', message: String((e && e.stack) || e) })); return; }
  if (m.cmd === 'ping') {
    pings.push({ seq: m.seq, sentPageMs: m.sentPageMs, receivedWorkerMs: +(performance.now() - T0).toFixed(1), engineStage: stage });
    post({ type: 'pong', seq: m.seq, sentPageMs: m.sentPageMs, workerMs: +(performance.now() - T0).toFixed(2), stage });
    return;
  }
  if (m.cmd === 'mode') { mode = m.mode; rate = m.rate || 1; post({ type: 'ack', cmd: 'mode', mode, rate }); if (idle) onIdle(); return; }
  if (!idle && ['mouse', 'key', 'advance', 'snapshot', 'timers', 'probetimer', 'end'].includes(m.cmd)) {
    post({ type: 'ack', cmd: m.cmd, seq: m.seq, r: -9, why: 'the engine is not waiting in idle' });
    return;
  }
  if (m.cmd === 'mouse') {
    moveClockForWall();
    const r = eng.instance.exports.fle_g3_mouse(m.kind, m.x, m.y, m.data | 0);
    post({ type: 'ack', cmd: 'mouse', seq: m.seq, r, kind: m.kind, x: m.x, y: m.y, t: +(performance.now() - T0).toFixed(2), sentPageMs: m.sentPageMs });
    if (r === 1) deliver(0);
    return;
  }
  if (m.cmd === 'key') {
    moveClockForWall();
    const r = eng.instance.exports.fle_g3_key(m.kind, m.vk, m.scan || 0, m.ext | 0);
    post({ type: 'ack', cmd: 'key', seq: m.seq, r, kind: m.kind, vk: m.vk, t: +(performance.now() - T0).toFixed(2), sentPageMs: m.sentPageMs });
    if (r === 1) deliver(0);
    return;
  }
  if (m.cmd === 'advance') {
    advanceTarget = idle.now + m.ms * 10000;
    post({ type: 'ack', cmd: 'advance', seq: m.seq, r: 1, targetTicks: advanceTarget });
    onIdle();
    return;
  }
  if (m.cmd === 'snapshot') {
    eng.instance.exports.fle_g3_snapshot();
    post({ type: 'snapDone', seq: m.seq, n: idle.n, now: idle.now });
    return;
  }
  if (m.cmd === 'probetimer') { const r = eng.instance.exports.fle_g3_probe_timer(m.lid, m.id, m.ms); post({ type: 'ack', cmd: 'probetimer', seq: m.seq, r }); return; }
  if (m.cmd === 'timers') { eng.instance.exports.fle_g3_timers(); post({ type: 'timersDone', seq: m.seq }); return; }
  if (m.cmd === 'end') { post({ type: 'ack', cmd: 'end', seq: m.seq, r: 0 }); deliver(-1); return; }
};

self.addEventListener('error', (e) => post({ type: 'jserror', message: String(e.message), filename: e.filename, lineno: e.lineno }));
self.addEventListener('unhandledrejection', (e) => post({ type: 'jserror', message: 'unhandledrejection: ' + String((e.reason && e.reason.stack) || e.reason).slice(0, 600) }));
self.g3State = () => ({ stage, ticks, idleCount, pings: pings.length, finished, mode, idle: idle ? { n: idle.n, now: idle.now, next: idle.next } : null });

async function run(c) {
  const root = c.root || '/data';
  const stdout = [], stderr = [];
  let exitCode = null, instance = null, memory = null;
  const t0 = performance.now();
  tickTimer = setInterval(() => { ticks++; }, 10);
  importScripts(c.glue);
  mark('glue_loaded');
  // an optional list of more seed files (data/seed/manifest.json: the host files of a VFS seed such as vfs-seed-idle1.tsv, tools/make-seed-pack.sh); a dist without it has the fonts-only seed
  let extra = [];
  try { const r = await fetch(c.dataBase + 'seed/manifest.json', { cache: 'no-store' }); if (r.ok) extra = (await r.json()).map((p) => 'seed/' + p); } catch (e) { extra = []; }
  const names = [FLE, ...MODULES, ...SEED, ...extra];
  const f0 = performance.now();
  const bufs = await Promise.all(names.map(async (n) => {
    const r = await fetch(c.dataBase + n, { cache: 'no-store' });
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
    FLE_FRUITY_NO_IDLE_STOP: '1',
  }, c.env || {});
  if (c.mode) mode = c.mode;
  if (c.rate) rate = c.rate;
  const opts = {
    arguments: [root + '/' + FLE, MODULES.map((m) => root + '/' + m).join(':')],
    print: (line) => { stdout.push(line); post({ type: 'stdout', line }); },
    printErr: (line) => { stderr.push(line); post({ type: 'stderr', line }); },
    onExit: (code) => {
      post({ type: 'quit', code, t: +(performance.now() - T0).toFixed(1) });
      if (exitCode === null) exitCode = code;
      mark('exit', { code });
      setTimeout(() => { if (!finished) { finished = true; finish(null); } }, 0);
    },
    fleProgress: (n) => post({ type: 'progress', entries: n, t: +(performance.now() - T0).toFixed(1) }),
    onAbort: (what) => { post({ type: 'stderr', line: 'onAbort: ' + what }); },
    onRuntimeInitialized: () => { stage = 'engine-running'; mark('main_start'); },
    fleIdle: (n, now, next) => {
      idleCount = n;
      stage = 'idle';
      idle = { n, now, next, wall: performance.now() };
      post({ type: 'idle', n, now, next, t: +(performance.now() - T0).toFixed(2) });
      onIdle();
    },
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
        const buf = await (await fetch(c.wasm, { cache: 'no-store' })).arrayBuffer();
        mark('wasm_downloaded', { bytes: buf.byteLength, ms: +(performance.now() - s).toFixed(1) });
        const s1 = performance.now();
        const mod = await WebAssembly.compile(buf);
        mark('wasm_compiled', { ms: +(performance.now() - s1).toFixed(1) });
        instance = await WebAssembly.instantiate(mod, imports);
        eng = { instance, opts };
        memory = instance.exports.memory;
        mark('wasm_instantiated', { memory_bytes: memory.buffer.byteLength });
        done(instance, mod);
      })().catch((e) => post({ type: 'error', phase: 'wasm', message: String((e && e.stack) || e) }));
      return {};
    },
    // the C side (harness/fruity_surface.c) calls this synchronously: kind 3 = a snapshot of every live surface. The pixels are BGRX, top-down, `stride` bytes per row.
    fleSurfaceEvent(kind, lid, pixels, w, h, stride) {
      const src = new Uint32Array(memory.buffer, pixels, (stride * h) >> 2);
      const rgba = new Uint32Array(w * h);
      const sw = stride >> 2;
      for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { const v = src[y * sw + x]; rgba[y * w + x] = 0xff000000 | ((v & 0xff) << 16) | (v & 0xff00) | ((v >>> 16) & 0xff); }
      let info = null;
      const ex = instance.exports, sc = ex.fle_surface_scratch();
      for (let i = 0; i < 32; i++) {
        if (ex.fle_surface_info(i, sc) !== 0) break;
        const d = new DataView(memory.buffer, sc, 96);
        if (d.getUint32(4, true) === lid) { info = { x: d.getInt32(16, true), y: d.getInt32(20, true), flags: d.getUint32(12, true), style: d.getUint32(56, true), exstyle: d.getUint32(60, true) }; break; }
      }
      post({ type: 'surface', kind, lid, w, h, info, data: rgba.buffer, t: +(performance.now() - T0).toFixed(1) }, [rgba.buffer]);
    },
  };

  async function finish(error) {
    mark('done');
    stage = 'finished';
    clearInterval(tickTimer);
    post({ type: 'done', exitCode, error, marks, workerTicks: ticks, pings, idleCount, memoryBytes: memory ? memory.buffer.byteLength : null, stdoutLines: stdout.length, ms: +(performance.now() - t0).toFixed(1), stdout, stderrTail: stderr.slice(-40) });
  }
  try { await self.Module(opts); post({ type: 'jserror', message: 'Module() resolved (exitCode ' + exitCode + ')' }); if (!finished) { finished = true; await finish(null); } } catch (e) {
    post({ type: 'jserror', message: 'Module() rejected: ' + String((e && e.stack) || e).slice(0, 800) });
    if (!(e && e.name === 'ExitStatus') && !finished) { finished = true; post({ type: 'stderr', line: 'trap/exception: ' + String((e && e.stack) || e).slice(0, 600) }); finish(String((e && e.stack) || e)).catch(() => {}); }
  }
}
