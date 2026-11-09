// The engine Worker (module worker). Owns the engine instance. The lifted FL engine runs in Module(). The runtime composes the windows into one picture of the screen (decision 0223) and this Worker shows it on the one
// canvas the page transferred (desk.ts); the page gets no pixel and knows no window. Input arrives through a SharedArrayBuffer ring (input/ring.ts), the cursor and the title as small messages.
import { sha256Hex } from '../bundle/sha256.ts';
import { classifyEngineError, FruityEngine, type FruityHost } from './fruity.ts';
import { hostPackFromBytes } from './hostfiles.ts';
import { IdleDriver, newDriverStats } from './idle-driver.ts';
import { IdleTrace } from './idle-trace.ts';
import { ReportThrottle } from './report-throttle.ts';
import { TaskbarReport } from './taskbar-report.ts';
import { watchContextLoss, type ContextLossTarget } from './ctx-resync.ts';
import type { FromWorker, ToWorker } from './protocol.ts';
import type { FileDlgAnswer, FileDlgRequest } from './filedlg.ts';
import { osStateKey, osStateOf } from './os-state.ts';
import { DeskPresenter, readDeskState, type DeskApi, type DeskSurface } from './desk.ts';
import { GlDeskPresenter, webgl2Usable, type DeskShow, type GlSurface, type SceneApi } from './desk-gl.ts';
import { RingReader, takenAtSchedulerPoint } from '../input/ring.ts';
import { QuietClock } from './desk-sleep.ts';
import { stopAudioReport } from '../audio/worker-hook.ts'; // decision 0175: listens for { cmd: 'audio-init' } (the page sends it)

declare const self: DedicatedWorkerGlobalScope;

// decision 0182: the time origin is constant; reading it at every call was one more property get per timestamp (there are three or four per idle)
const TIME_ORIGIN = performance.timeOrigin;
const nowAbs = (): number => TIME_ORIGIN + performance.now();
const post = (m: FromWorker, transfer: Transferable[] = []): void => self.postMessage(m, transfer);
const log = (level: 'info' | 'warn' | 'error', text: string): void => post({ t: 'log', level, text });
const mark = (name: string, detail?: Record<string, unknown>): void => post({ t: 'mark', name, abs: nowAbs(), ...(detail ? { detail } : {}) });

let fruity: FruityEngine | null = null;
let driver: IdleDriver | null = null;
const reports = new ReportThrottle();
const trace = new IdleTrace();
const checkpoints: { n: number; hash: number }[] = [];
let lastAppSeq = -1;
// decision 0223: the one canvas of the desktop (the runtime composes the windows into one picture; this Worker shows it) and the input ring. Nothing below knows about windows.
let deskCanvas: OffscreenCanvas | null = null;
let deskPresenter: DeskShow | null = null;
let ring: RingReader | null = null;
const DESK_FALLBACK_MS = 20;
/** desk2: when no present changed a pixel for this long, the Worker stops asking for animation frames and looks for a change from the engine's idle (at most every DESK_FALLBACK_MS); the first change wakes the frame loop */
const DESK_SLEEP_AFTER_MS = 100;
const deskCounters = { ringReads: 0, wakes: 0, ticks: 0, idleTicks: 0 };

const curMemory = (): WebAssembly.Memory | null => fruity?.memory ?? null;

self.onmessage = (ev: MessageEvent<ToWorker>) => {
  const m = ev.data;
  void handle(m).catch((e: unknown) => post({ t: 'error', phase: m.cmd, message: String((e as Error)?.stack ?? e) }));
};

async function handle(m: ToWorker): Promise<void> {
  switch (m.cmd) {
    case 'init': await init(m); break;
    case 'start': void start(); break;
    case 'end-idle': if (fruity && !(driver ? driver.end() : fruity.endIdle())) log('warn', 'the engine is not idle: nothing to end'); break;
    case 'input': onInput(m.seq, m.tPage, m.ev); break;
    case 'filedlg-answer': onFileDlgAnswer(m.id, m.answer); break;
    case 'readback': await readback(m.key); break;
    case 'wake': deskCounters.wakes++; drainRing(); break;
    case 'desk-dump': deskDump(false); break;
    case 'desk-peek': deskDump(true); break;
    case 'desk-gl-read': deskGlRead(); break;
    case 'ping': post({ t: 'pong', id: m.id }); break;
  }
}

async function init(m: Extract<ToWorker, { cmd: 'init' }>): Promise<void> {
  deskCanvas = m.desk.canvas;
  ring = new RingReader(m.desk.ring);
  watchContextLoss(deskCanvas as unknown as ContextLossTarget, { lost: () => { deskPresenter?.resync(); wakeDesk(); }, restored: () => { deskPresenter?.resync(); wakeDesk(); } });   // a sleeping frame loop (desk2) is woken: the whole picture is sent again
  if (!m.assets.glue || !m.assets.dll || !m.assets.seed) throw new Error('the emscripten engine needs the glue, the seed pack and the DLL pack');
  const host: FruityHost = {
    mark,
    stdout: (line) => pushLog('out', line),
    stderr: (line) => pushLog('err', line),
    log,
    idle: onEngineIdle,
    fileDlg: (id, request) => { mark('filedlg_request', { id }); post({ t: 'filedlg-request', id, request: request as FileDlgRequest }); },
    fileDlgProgress: (id, done, total) => post({ t: 'filedlg-progress', id, done, total }),
    inputPoll,
    inputDone,
  };
  // the engine reads its packs from the bytes the page loaded (decision 0221: the libc file calls are answered from them)
  const dllPack = hostPackFromBytes(m.assets.dll);
  const seedPack = hostPackFromBytes(m.assets.seed);
  mark('pack_source_dll', { how: 'memory' });
  mark('pack_source_seed', { how: 'memory' });
  fruity = await FruityEngine.create(m.manifest, { module: m.assets.module, glue: m.assets.glue, seed: seedPack, dll: dllPack }, host);
  log('info', `engine ${m.manifest.bundle.name} ${m.manifest.bundle.version} ready; stack switching ${fruity.canSuspend ? 'JSPI' : 'unavailable'}`);
  post({ t: 'ready', kind: m.manifest.engine.kind });
}

// ---- input (decision 0141): the page's raw events reach the IdleDriver; the engine takes them at its idle ----
let lagCutTicks = 0;
let lastLagLog = -1e9;
// the engine instance exists only once run() has instantiated the wasm, so the driver is made at the first idle or the first input
function ensureDriver(): IdleDriver | null {
  if (driver || !fruity) return driver;
  const input = fruity.inputApi;
  if (!input) return null;
  // a MessageChannel turn has no nested-timer clamp (setTimeout nested more than 5 deep waits at least 4 ms); one bounce is pending at a time
  const turns = new MessageChannel();
  let bounce: (() => void) | null = null;
  turns.port1.onmessage = () => { const f = bounce; bounce = null; f?.(); };
  driver = new IdleDriver(input, {
    deliver: (code) => fruity?.deliver(code) ?? false,
    now: () => nowAbs(),
    setTimer: (fn, ms) => setTimeout(fn, ms),
    clearTimer: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
    yieldTurn: (fn) => { bounce = fn; turns.port2.postMessage(0); },
  }, {
    processed: (list, tIdle) => {
      flushLogs();
      post({ t: 'input-done', items: list.map((x) => ({ seq: x.seq, tPage: x.tPage, tInject: x.tInject, tIdle })) });
      // decision 0223: what the engine drew in answer to the input is shown now, not at the next animation frame (S-desk-input-present); the compositor holds the frame until the display's next refresh either way
      if (deskPresenter) { try { const shown = deskTick(false); flushDeskStats(true, shown ? list.map((x) => x.seq) : undefined); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); } }
    },
    dropped: (seq, reason) => post({ t: 'input-dropped', seq, reason }),
    resumed: (_code, abs, why) => { ring?.setParked(false); if (reports.resume(why)) post({ t: 'resume', abs, why }); },
    // reported here, before the driver decides what the engine does next: the page must see this idle before the resume that may follow it at once (decision 0170: the next deadline is often already due)
    idle: (n, now, next, busyMs, at) => { ring?.setParked(true); reportIdle(n, now, next, busyMs, at); },
    lagCut: (ticks) => {
      lagCutTicks += ticks;
      const at = performance.now();
      if (at - lastLagLog > 5000) { lastLagLog = at; log('warn', `the virtual clock fell ${(lagCutTicks / 10000).toFixed(0)} ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)`); lagCutTicks = 0; }
    },
  });
  return driver;
}

function onInput(seq: number, tPage: number, ev: import('../input/events.ts').RawInput): void {
  ensureDriver();
  if (!driver) { post({ t: 'input-dropped', seq, reason: 'this engine build has no input exports (it was built before decision 0136)' }); return; }
  driver.submit(ev, seq, tPage);
}

// ---- decision 0280 (PROVISIONAL): the engine's scheduler reads the input ring itself (lift/harness/fruity_idle.c fle_input_ring_poll; the import is called at every scheduler point, about 2,000 times a second while the engine runs) ----
// Before: an input waited in the ring until the engine reached an idle (the Worker's event loop ran only then), 133 ms (median, a click during playback) when the mixer is behind real time. Now the runtime takes the records at its next
// scheduler point and queues them with the functions the exports use; this file stages the records into the runtime's buffer and keeps the books; the answer (a present that changed a pixel) is tried every INLINE_TRY_MS from here
// too, since the Worker's own turn comes only at the idle. Nothing here calls a wasm export except deskTick (the present), which the idle path calls as well.
const RING_REC = 8, INLINE_TRY_MS = 6, INLINE_GIVEUP_MS = 150;
let inlineStaged: { seq: number; tPage: number; ev: import('../input/events.ts').RawInput }[] = [];
let inlineTryAt = 0;
let inlineSince = 0;
// decision 0280, L1c (PROVISIONAL): which present may answer an inline input. The runtime writes fle_input_settled() into the word after the records (fruity_idle.c): 1 = the guest has taken every event the page sent out of its queue, has retrieved every message made of
// them, and the thread that did waits in its message loop again. An input is answered only by a present that was started after a scheduler point that reported 1 (the picture then holds what the guest drew for it); a present before that, which shows
// only another change (the playhead), does not answer. No class name, no window kind.
let inlineSettled = false, settledBuf: ArrayBuffer | null = null, settledView: Int32Array | null = null;
function readSettled(ptr: number, max: number): number {
  const mem = fruity?.memory;
  if (!mem) return 0;
  if (settledBuf !== mem.buffer) { settledBuf = mem.buffer; settledView = new Int32Array(settledBuf); }
  return (settledView as Int32Array)[(ptr >> 2) + max * RING_REC] ?? 0;
}
function inputPoll(ptr: number, max: number): number {
  const d = driver, mem = fruity?.memory;
  if (!ring || !d || !d.ready || !mem) return 0;
  if (d.awaitingCount > 0 && inlineSince !== 0 && !inlineSettled && readSettled(ptr, max) === 1) inlineSettled = true;
  if (ring.pending === 0) { if (d.awaitingCount > 0) answerInline(); return 0; }
  let i32: Int32Array | null = null;
  inlineStaged = [];
  const n = ring.drainN(max, (r) => {
    const o = inlineStaged.length * RING_REC, e = r.ev;
    if (!takenAtSchedulerPoint(e)) return false;   // syscmd and any later kind stay in the ring (with what follows them) for the IdleDriver at the next idle
    if (i32 === null) i32 = new Int32Array(mem.buffer, ptr, max * RING_REC);
    if (e.k === 'mouse') { i32[o] = 1; i32[o + 1] = e.kind; i32[o + 2] = e.x; i32[o + 3] = e.y; i32[o + 4] = e.data; }
    else if (e.k === 'key') { i32[o] = 2; i32[o + 1] = e.kind; i32[o + 2] = e.vk; i32[o + 3] = e.scan; i32[o + 4] = e.ext; }
    else { i32[o] = 3; i32[o + 1] = e.kind; i32[o + 2] = e.width; i32[o + 3] = e.height; i32[o + 4] = 0; }
    i32[o + 5] = r.seq; i32[o + 6] = 0; i32[o + 7] = 0;
    inlineStaged.push(r);
  });
  deskCounters.ringReads++;
  if (n === 0 && d.awaitingCount > 0) answerInline();   // a record that stays in the ring (a syscmd waiting for the idle) must not stop the answers of the inputs taken before it
  return n;
}
function inputDone(ptr: number, n: number): void {
  const d = driver, mem = fruity?.memory;
  if (!d || !mem) { inlineStaged = []; return; }
  const i32 = new Int32Array(mem.buffer, ptr, n * RING_REC), at = nowAbs();
  const done: { seq: number; tPage: number; tInject: number; ev: import('../input/events.ts').RawInput }[] = [];
  for (let i = 0; i < n; i++) {
    const r = inlineStaged[i];
    if (!r) continue;
    if (i32[i * RING_REC + 6] === 1) done.push({ seq: r.seq, tPage: r.tPage, tInject: at, ev: r.ev });
    else post({ t: 'input-dropped', seq: r.seq, reason: 'the engine refused the event (its input queue is full)' });
  }
  inlineStaged = [];
  if (done.length > 0) { if (d.awaitingCount === 0) inlineSince = performance.now(); inlineSettled = false; d.injectedInline(done); }
}
// the answer to the inputs the runtime took inline: the first present that changed a pixel after the injection, by whichever path made it (this function, the rAF tick, the idle's fallback present: deskTick calls completeInline when it shows
// something and inputs await); this function tries a present every INLINE_TRY_MS, and after INLINE_GIVEUP_MS without a change the inputs are finished without an answer (an input that changes no pixel)
function completeInline(shown: boolean): void {
  const d = driver;
  if (!d || d.awaitingCount === 0 || inlineSince === 0) return;
  const list = d.takeAwaiting(), at = nowAbs();
  inlineSince = 0;
  inlineSettled = false;
  post({ t: 'input-done', items: list.map((x) => ({ seq: x.seq, tPage: x.tPage, tInject: x.tInject, tIdle: at })) });
  flushDeskStats(true, shown ? list.map((x) => x.seq) : undefined);
}
function answerInline(): void {
  const d = driver;
  if (!d || d.awaitingCount === 0 || !deskPresenter || inlineSince === 0) return;
  const t = performance.now();
  if (!inlineSettled) { if (t - inlineSince >= INLINE_GIVEUP_MS) completeInline(false); return; }
  if (t - inlineTryAt < INLINE_TRY_MS) return;
  inlineTryAt = t;
  try { deskTick(false); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
  if (d.awaitingCount > 0 && performance.now() - inlineSince >= INLINE_GIVEUP_MS) completeInline(false);
}

// what the page is told about an idle (ReportThrottle: the first, then one per 40 ms; decision 0170)
function reportIdle(n: number, nowTicks: number, nextTicks: number, busyMs: number | null, abs: number): void {
  if (!reports.idle(n, abs)) return;
  const caps = { input: driver !== null, appview: fruity?.hasAppView ?? false, display: fruity?.hasDisplay ?? false };
  post({ t: 'idle', n, now: nowTicks, next: nextTicks, busyMs, abs, caps, stats: driver?.stats ?? { ...newDriverStats(), idleCount: n }, queued: driver?.queued ?? 0, ...(checkpoints.length ? { trace: checkpoints.splice(0) } : {}), memBytes: curMemory()?.buffer.byteLength, ...(fruity?.hostFileStats ? { hostFiles: fruity.hostFileStats } : {}) });
}

// the engine is waiting: show what it drew (the desktop picture), then let the driver decide (give queued input, or wait for the engine's next deadline)
function onEngineIdle(n: number, nowTicks: number, nextTicks: number): void {
  const ck = trace.push(n, nowTicks, nextTicks);
  if (ck) checkpoints.push(ck);
  flushLogs();
  startDeskLoop();
  // the Worker's requestAnimationFrame is not delivered while the engine keeps the Worker busy (measured, when the engine runs behind the wall clock): the picture is then shown from here, at most every DESK_FALLBACK_MS
  if (performance.now() - lastTickAt > DESK_FALLBACK_MS) { lastTickAt = performance.now(); deskCounters.idleTicks++; try { deskTick(); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); } }
  if (saveArmed) pollSavedFiles();
  ensureDriver();
  drainRing(); // the page's input waits in the ring: it is queued now and given to the engine by onIdle below
  if (driver) driver.onIdle(n, nowTicks, nextTicks);
  else reportIdle(n, nowTicks, nextTicks, null, nowAbs());
  drainRing(); // an input that was written after the first look but before this idle was announced as parked
}

// ---- decision 0193: the engine's file dialogs. The engine is suspended in Show() (JSPI) while the page asks the person; the answer arrives as a message, the library reads the File into the engine's memory and resumes it ----
let saveArmed = false;
function onFileDlgAnswer(id: number, answer: FileDlgAnswer): void {
  if (!fruity) return;
  if (answer.ok && answer.items.some((i) => i.role === 'S')) saveArmed = true;
  const taken = fruity.answerFileDlg(id, answer);
  mark('filedlg_answer', { id, taken, ok: answer.ok });
  if (!taken) log('warn', `the page answered file dialog ${id}, which the engine is not waiting for`);
}
// at the engine's idle: a Save target whose file the engine has finished writing goes to the page as a copy (never to a server; the page offers it as a download)
function pollSavedFiles(): void {
  for (let k = 0; k < 8 && fruity; k++) {
    const f = fruity.pollSaved();
    if (!f) return;
    const name = f.guestPath.slice(f.guestPath.lastIndexOf('\\') + 1);
    post({ t: 'saved-file', guestPath: f.guestPath, name, size: f.size, data: f.data.buffer as ArrayBuffer }, [f.data.buffer as ArrayBuffer]);
  }
}

// ---- the engine's stdout / stderr, batched (the engine prints about 1,600 lines; a message per line would be noise) ----
const pending: { out: string[]; err: string[] } = { out: [], err: [] };
let flushTimer: ReturnType<typeof setTimeout> | null = null;
function pushLog(stream: 'out' | 'err', line: string): void {
  pending[stream].push(line);
  if (flushTimer === null) flushTimer = setTimeout(flushLogs, 100);
}
function flushLogs(): void {
  if (flushTimer !== null) { clearTimeout(flushTimer); flushTimer = null; }
  for (const stream of ['out', 'err'] as const) {
    if (pending[stream].length === 0) continue;
    post({ t: 'engine-log', stream, lines: pending[stream] });
    pending[stream] = [];
  }
}

async function start(): Promise<void> {
  if (!fruity) throw new Error('start before init');
  await startFruity(fruity);
}

async function startFruity(f: FruityEngine): Promise<void> {
  mark('engine_start');
  const result = await f.run();
  stopAudioReport();
  flushLogs();
  try { deskTick(); } catch (e) { log('error', `final desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
  flushDeskStats(true);
  const drawn = deskPresenter?.stats.presents ?? 0;
  mark('worker_done', { memoryBytes: curMemory()?.buffer.byteLength ?? null, drawn, hostFiles: f.hostFileStats });
  if (result.kind === 'trap') {
    const c = classifyEngineError(result.message);
    post({ t: 'exit', reason: 'trap', code: -1, message: `${c.what} (${c.kind})\n${result.message.slice(0, 1200)}`, presents: drawn });
  } else if (result.kind === 'exit' && result.code !== 0) {
    post({ t: 'exit', reason: 'stopped', code: result.code, message: `the engine exited with code ${result.code}`, presents: drawn });
  } else {
    post({ t: 'exit', reason: 'finished', code: result.code ?? 0, message: result.kind === 'returned' ? 'Module() returned without an exit code' : 'the engine exited with code 0', presents: drawn });
  }
}

function deskGlRead(): void {
  const p = deskPresenter;
  if (!(p instanceof GlDeskPresenter)) { post({ t: 'desk-dump-error', message: 'the desktop is not shown with WebGL (the canvas gave no WebGL2 context)' }); return; }
  const r = p.readPixels();
  if (!r) { post({ t: 'desk-dump-error', message: 'no scene has been drawn yet' }); return; }
  const data = r.data.buffer as ArrayBuffer;
  post({ t: 'desk-gl-pixels', width: r.width, height: r.height, data }, [data]);
}

async function readback(key: string): Promise<void> {
  if (key !== 'desk' || !deskCanvas) { post({ t: 'readback-error', key, message: `no OffscreenCanvas bound to ${key}` }); return; }
  if (deskPresenter instanceof GlDeskPresenter) {
    const r = deskPresenter.readPixels();
    if (!r) { post({ t: 'readback-error', key, message: 'no scene has been drawn yet' }); return; }
    post({ t: 'readback', key, width: r.width, height: r.height, sha256: await sha256Hex(r.data) });
    return;
  }
  const ctx = deskCanvas.getContext('2d') as OffscreenCanvasRenderingContext2D;
  const d = ctx.getImageData(0, 0, deskCanvas.width, deskCanvas.height);
  post({ t: 'readback', key, width: deskCanvas.width, height: deskCanvas.height, sha256: await sha256Hex(new Uint8Array(d.data.buffer)) });
}

// ---- decision 0223: the desktop ----
// The input: the page writes raw events into the ring (input/ring.ts); they are read here at every idle of the engine and when the page wakes the Worker. Nothing else changed in how the engine is given input.
function drainRing(): void {
  if (!ring) return;
  ensureDriver();
  if (!driver) return;
  const n = ring.drain((r) => driver?.submit(r.ev, r.seq, r.tPage));
  if (n) deskCounters.ringReads++;
}

// The picture: once per animation frame of the Worker (requestAnimationFrame of a dedicated Worker; setTimeout where it is missing), the compositor of the runtime is asked to bring the picture up to date and the
// changed rectangles are put on the canvas (desk.ts). The same tick reads the operating system's state that is not a picture (the cursor, the title of the foreground window) and reports it when it changed.
let deskLoopOn = false;
let firstDeskPixel = false;
const presentedAt: number[] = [];
let lastDeskReport = 0;
let lastTickAt = 0;
let lastOs = '';
let lastTitle = '';
const taskbarReport = new TaskbarReport();
const deskQuiet = new QuietClock(DESK_SLEEP_AFTER_MS, performance.now());
let deskNextFrame: (() => void) | null = null;
function startDeskLoop(): void {
  if (deskLoopOn) return;
  deskLoopOn = true;
  const g = self as unknown as { requestAnimationFrame?: (cb: (t: number) => void) => number };
  const next = (): void => { if (g.requestAnimationFrame) g.requestAnimationFrame(tick); else setTimeout(tick, 16); };
  const tick = (): void => {
    deskCounters.ticks++;
    const now = performance.now();
    lastTickAt = now;
    try { deskTick(); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
    // nothing changed for DESK_SLEEP_AFTER_MS: no more animation frames until the picture changes (the engine's idle looks for a change at most every DESK_FALLBACK_MS and wakes the loop; a Worker that asks for a frame every
    // refresh of the display keeps the compositor and the GPU process working for a picture that does not move: measured with the refresh rate unlimited, and at 240 Hz)
    if (deskQuiet.sleepAfterFrame(now)) return;
    next();
  };
  deskNextFrame = next;
  deskQuiet.wake(performance.now());
  next();
}
function wakeDesk(): void {
  if (deskQuiet.wake(performance.now())) deskNextFrame?.();
}
/**
 * Decision 0240: how the desktop is shown is decided here, once, by what the browser can do - not by a URL parameter or a setting. When the engine describes its scene (fle_desk_scene) and the canvas gives a WebGL2 context, the
 * description is drawn with WebGL2 (desk-gl.ts); otherwise the runtime composes one picture and putImageData shows it (desk.ts). The two never run together: the runtime is switched to the scene only after the GL context exists.
 */
function makeDeskPresenter(api: DeskApi): DeskShow {
  const memory = (): ArrayBufferLike | null => fruity?.memory?.buffer ?? null;
  if (api.scene && api.sceneMode && webgl2Usable(() => new OffscreenCanvas(1, 1) as unknown as GlSurface)) {
    try {
      const gl = new GlDeskPresenter(deskCanvas as unknown as GlSurface, api as SceneApi, memory);
      const target = deskCanvas as unknown as { addEventListener(type: string, listener: (ev: Event) => void): void };
      target.addEventListener('webglcontextlost', (ev) => { ev.preventDefault(); gl.setLost(true); });
      target.addEventListener('webglcontextrestored', () => { gl.setLost(false); wakeDesk(); });
      log('info', 'display method D: the runtime describes the desktop (windows, positions, written rectangles) and this Worker draws it with WebGL2, one texture per window');
      return gl;
    } catch (e) {
      log('info', `display method D: no WebGL2 for the desktop canvas (${String((e as Error)?.message ?? e)}); the runtime composes one picture and this Worker shows it with putImageData`);
    }
  } else log('info', 'display method D: no scene description in this engine build (fle_desk_scene) or no usable WebGL2 (a context with a largest texture of at least 8192); the runtime composes one picture and this Worker shows it with putImageData');
  return new DeskPresenter(deskCanvas as unknown as DeskSurface, api, memory);
}
/** one present; true when it changed a pixel. `flush`: tell the page about it now unless it was told less than 100 ms ago */
function deskTick(flush = true): boolean {
  if (!fruity || !deskCanvas) return false;
  if (!deskPresenter) {
    const api = fruity.deskApi;
    if (!api) { log('error', 'display method D: this engine build has no desktop compositor'); deskLoopOn = false; return false; }
    deskPresenter = makeDeskPresenter(api);
  }
  const gate = inlineSettled;   // decision 0280, L1c: read before the present: the picture of this present is composed after it
  const shown = deskPresenter.present();
  if (shown > 0) {
    presentedAt.push(nowAbs());
    if (deskQuiet.changed(performance.now())) deskNextFrame?.();
    if (gate) completeInline(true);   // decision 0280: inputs the runtime took inline are answered by the first present that shows something
    if (!firstDeskPixel) { firstDeskPixel = true; mark('first_pixel', { source: 'desktop', width: deskCanvas.width, height: deskCanvas.height }); }
  }
  const v = fruity.readAppView(lastAppSeq);
  if (v) {
    lastAppSeq = v.seq;
    const os = osStateOf(v, lastTitle);
    lastTitle = os.title;
    const key = osStateKey(os);
    if (key !== lastOs) { lastOs = key; post({ t: 'osstate', ...os, abs: nowAbs() }); }
  }
  // decision 0272: a minimized window with a taskbar button, as the engine says it (null: this build has no such export; nothing is posted)
  const tb = taskbarReport.next(fruity.taskbarIconic());
  if (tb !== null) post({ t: 'taskbar', lid: tb });
  if (flush) flushDeskStats(false);
  return shown > 0;
}
function flushDeskStats(force: boolean, answers?: number[]): void {
  const now = performance.now();
  if (!force && now - lastDeskReport < 100) return;
  if (!deskPresenter || (presentedAt.length === 0 && !force && now - lastDeskReport < 1000)) return;
  lastDeskReport = now;
  post({ t: 'desk', stats: { ...deskPresenter.stats, ringReads: deskCounters.ringReads, ringFull: 0, wakes: deskCounters.wakes, ticks: deskCounters.ticks, idleTicks: deskCounters.idleTicks, sleeps: deskQuiet.sleeps }, at: presentedAt.splice(0, 64), ...(answers && answers.length > 0 ? { answers } : {}), nWindows: 0 });
}

// A test aid (any display method): the compositor is asked for the whole desktop as the window table is now, and the picture is copied out. With display method D the next present shows everything again as well.
function deskDump(peek: boolean): void {
  const api = fruity?.deskApi, mem = fruity?.memory;
  if (!api || !mem) { post({ t: 'desk-dump-error', message: 'this engine build has no desktop compositor' }); return; }
  // peek: the picture as it is (what the presenter last read), no compose; dump: the whole picture drawn again from the window table (the next present then shows everything)
  if (peek && deskPresenter instanceof GlDeskPresenter) {   // decision 0240: with WebGL the picture 'as it is' is the scene drawn again from the textures the browser holds, read back
    const r = deskPresenter.readPixels();
    if (!r) { post({ t: 'desk-dump-error', message: 'the compositor has no picture yet' }); return; }
    const copy = r.data.buffer as ArrayBuffer;
    post({ t: 'desk-dump', width: r.width, height: r.height, data: copy, windows: api.describe() }, [copy]);
    return;
  }
  let st = peek ? deskPresenter?.lastState ?? null : null;
  if (!peek) { api.resync(); const ptr = api.compose(); st = readDeskState(mem.buffer, ptr); }   // the buffer after compose: it can grow the memory (see DeskPresenter.present)
  if (!st) { post({ t: 'desk-dump-error', message: 'the compositor has no picture yet' }); return; }
  const data = new Uint8Array(mem.buffer, st.pixels, st.height * st.stride).slice().buffer as ArrayBuffer;   // decision 0271: a SharedArrayBuffer's slice() is a SharedArrayBuffer, which cannot be in the transfer list (DataCloneError in Firefox without WebGL); a Uint8Array's slice() copies into a plain ArrayBuffer
  if (!peek) api.resync();
  post({ t: 'desk-dump', width: st.width, height: st.height, data, windows: api.describe() }, [data]);
}
