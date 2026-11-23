// The engine Worker (module worker). Owns the engine instance. The lifted FL engine runs in Module(). The runtime composes the windows into one picture of the screen (decision 0223). Since C-2 this Worker only
// composes: it says to the render Worker (render-worker.ts, over a MessagePort) that the picture changed, and that Worker draws it from the engine's shared memory (desk-link.ts is the handshake). The page gets no pixel and knows no window.
// Input arrives through a SharedArrayBuffer ring (input/ring.ts), the cursor and the title as small messages.
import { classifyEngineError, FruityEngine, type FruityHost } from './fruity.ts';
import { hostPackFromBytes } from './hostfiles.ts';
import { LazyFileSource } from './lazy-file.ts';
import { IdleDriver, newDriverStats } from './idle-driver.ts';
import { IdleTrace } from './idle-trace.ts';
import { ReportThrottle } from './report-throttle.ts';
import { TaskbarReport } from './taskbar-report.ts';
import type { EngineToRender, FromWorker, RenderToEngine, ToWorker } from './protocol.ts';
import type { FileDlgAnswer, FileDlgRequest } from './filedlg.ts';
import { osStateKey, osStateOf } from './os-state.ts';
import { deskGen, readDeskState, type DeskApi, type DeskState } from './desk.ts';
import { sceneGen } from './desk-gl.ts';
import { DeskGate } from './desk-link.ts';
import { RingReader, takenAtSchedulerPoint } from '../input/ring.ts';
import { stopAudioReport } from '../audio/worker-hook.ts'; // decision 0175: listens for { cmd: 'audio-init' } (the page sends it)

declare const self: DedicatedWorkerGlobalScope;

// decision 0182: the time origin is constant; reading it at every call was one more property get per timestamp (there are three or four per idle)
const TIME_ORIGIN = performance.timeOrigin;
const nowAbs = (): number => TIME_ORIGIN + performance.now();
const post = (m: FromWorker, transfer: Transferable[] = []): void => self.postMessage(m, transfer);
const log = (level: 'info' | 'warn' | 'error', text: string): void => post({ t: 'log', level, text });
const mark = (name: string, detail?: Record<string, unknown>): void => post({ t: 'mark', name, abs: nowAbs(), ...(detail ? { detail } : {}) });

/** Issue #1314: how long a read of a lazy file waits for its bytes before it fails (lazy-file.ts) */
const LAZY_TIMEOUT_MS = 120000;
let fruity: FruityEngine | null = null;
let driver: IdleDriver | null = null;
const reports = new ReportThrottle();
const trace = new IdleTrace();
const checkpoints: { n: number; hash: number }[] = [];
let lastAppSeq = -1;
// decision 0223: the input ring. The desktop: the picture is composed here and drawn by the render Worker (C-2): `port` carries the messages, `gate` the handshake. Nothing below knows about windows.
let ring: RingReader | null = null;
let port: MessagePort | null = null;
let gate: DeskGate | null = null;
const deskCounters = { ringReads: 0, wakes: 0 };
/** C-2: the compositor's API, once the wasm is instantiated; `deskReady`: the render Worker has been told that there is one */
let deskApi: DeskApi | null = null;
let deskReady = false;
let deskMissingLogged = false;
/** the scene mode (decision 0240) is on: the description is composed with scene() and drawn by the render Worker with WebGL2 */
let deskScene = false;
/** the address of the compositor's state (or of the scene) at the last compose, and the composes' time not yet told to the render Worker */
let deskPtr = 0;
let pendingComposeMs = 0;
let framesSent = 0;
let pendingDump: string | null = null;
let sharedWarned = false;

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
    case 'wake': deskCounters.wakes++; drainRing(); break;
    case 'desk-dump': deskDump(false); break;
    case 'desk-peek': deskDump(true); break;
    case 'window-api-trace': fruity?.windowApiTrace(m.on); break;
    case 'ping': post({ t: 'pong', id: m.id }); break;
  }
}

async function init(m: Extract<ToWorker, { cmd: 'init' }>): Promise<void> {
  ring = new RingReader(m.desk.ring);
  port = m.desk.port;
  port.onmessage = (ev: MessageEvent<RenderToEngine>) => onRender(ev.data);
  gate = new DeskGate(m.desk.ctl);
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
  // Issue #1314: the lazy files are in the host files from the start; a read that comes before their bytes waits for them (lazy-file.ts)
  const lazy = (m.assets.lazy ?? []).map((l) => new LazyFileSource(l, { timeoutMs: LAZY_TIMEOUT_MS, onWait: (path, ms) => mark('lazy_wait', { path, ms: Math.round(ms) }) }));
  fruity = await FruityEngine.create(m.manifest, { module: m.assets.module, glue: m.assets.glue, seed: seedPack, dll: dllPack, lazy }, host);
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
      if (deskReady) { try { deskTick(false, list.map((x) => x.seq)); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); } }
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
  d.syncRunningClock();
  serviceDeskRequest();
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
// the answer to the inputs the runtime took inline: the first compose that changed a pixel after the injection, by whichever path made it (this function, the render Worker's question: deskTick calls completeInline when it changes
// something and inputs await); this function tries a compose every INLINE_TRY_MS, and after INLINE_GIVEUP_MS without a change the inputs are finished without an answer (an input that changes no pixel)
/** the inputs the runtime took inline are done; `shown`: the present that answers them showed a change (their sequence numbers are returned, for the render Worker's latency report) */
function completeInline(shown: boolean): number[] | undefined {
  const d = driver;
  if (!d || d.awaitingCount === 0 || inlineSince === 0) return undefined;
  const list = d.takeAwaiting(), at = nowAbs();
  inlineSince = 0;
  inlineSettled = false;
  post({ t: 'input-done', items: list.map((x) => ({ seq: x.seq, tPage: x.tPage, tInject: x.tInject, tIdle: at })) });
  return shown ? list.map((x) => x.seq) : undefined;
}
function answerInline(): void {
  const d = driver;
  if (!d || d.awaitingCount === 0 || !deskReady || inlineSince === 0) return;
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

// the engine is waiting: then let the driver decide (give queued input, or wait for the engine's next deadline). The picture is not composed here (C-2): the render Worker asks for it (onRender 'want'), and its own frame loop
// is not held back by the engine's business, which is what the idle used to do with a compose every 20 ms while nothing moved
function onEngineIdle(n: number, nowTicks: number, nextTicks: number): void {
  const ck = trace.push(n, nowTicks, nextTicks);
  if (ck) checkpoints.push(ck);
  flushLogs();
  sendDeskReady();
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
  try { deskTick(false); } catch (e) { log('error', `final desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
  const drawn = framesSent;
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

// ---- decision 0223: the desktop (the picture is drawn by the render Worker, render-worker.ts; C-2) ----
// The input: the page writes raw events into the ring (input/ring.ts); they are read here at every idle of the engine and when the page wakes the Worker. Nothing else changed in how the engine is given input.
function drainRing(): void {
  if (!ring) return;
  ensureDriver();
  if (!driver) return;
  const n = ring.drain((r) => driver?.submit(r.ev, r.seq, r.tPage));
  if (n) deskCounters.ringReads++;
}

// The picture (C-2): the compositor of the runtime is asked to bring the picture up to date when the render Worker asks (onRender 'want'), and when an input is answered (answerInline, the processed inputs).
// A compose changes the picture in the engine's memory and the render Worker reads what changed from there (render-worker.ts). Nothing is composed while the render Worker has not read the last picture (gate.busy).
// The operating system's state that is not a picture (the cursor, the title of the foreground window, the taskbar) is read at each compose, as before.
let lastOs = '';
let lastTitle = '';
const taskbarReport = new TaskbarReport();

function renderPost(m: EngineToRender, transfer: Transferable[] = []): void { port?.postMessage(m, transfer); }

/** the compositor's API (decision 0223; scene: decision 0240), once the wasm is instantiated */
function compositor(): DeskApi | null {
  if (deskApi) return deskApi;
  deskApi = fruity?.deskApi ?? null;
  if (!deskApi && fruity?.instance && !deskMissingLogged) { deskMissingLogged = true; log('error', 'display method D: this engine build has no desktop compositor'); }
  return deskApi;
}

/** tells the render Worker once that there is a compositor, and whether its description is to be drawn with WebGL (the scene exports exist) */
function sendDeskReady(): void {
  if (deskReady || !port) return;
  const api = compositor();
  if (!api) return;
  deskReady = true;
  renderPost({ t: 'desk-ready', scene: !!(api.scene && api.sceneMode) });
}

/** the render Worker's messages: its question (compose now), the resync, the scene mode, and the answer to a test aid's read-pixels */
function onRender(m: RenderToEngine): void {
  switch (m.cmd) {
    case 'want': serviceDeskRequest(); break;
    case 'resync': compositor()?.resync(); break;
    case 'scene-mode': { const api = compositor(); if (api && api.scene && api.sceneMode && !deskScene) { api.sceneMode(1); deskScene = true; } break; }
    case 'pixels': { const d = pendingDump; pendingDump = null; if (d !== null) post({ t: 'desk-dump', width: m.width, height: m.height, data: m.data, windows: d }, [m.data]); break; }
    case 'pixels-error': pendingDump = null; post({ t: 'desk-dump-error', message: m.message }); break;
  }
}

// The shared request is consumed once, either here in the event loop or at a
// guest scheduler point. A wake message arriving afterwards does no extra work.
function serviceDeskRequest(): void {
  if (!gate?.takeRequest()) return;
  sendDeskReady();
  try { deskTick(true); }
  catch (e) {
    log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`);
    renderNoChange();
  }
}

/** one compose (the picture brought up to date) and the hand-over to the render Worker: when the picture changed, it is told, with the inputs this change answers (`seqs`, and the inline ones); `reply`: the render Worker asked, so it is answered even when nothing changed. True when the picture changed */
function deskTick(reply: boolean, seqs?: number[]): boolean {
  const api = compositor(), mem = fruity?.memory;
  if (!api || !mem || !gate) { if (reply) renderNoChange(); return false; }
  if (gate.busy) { if (reply) renderNoChange(); return false; }
  const gate0 = inlineSettled;   // decision 0280, L1c: read before the compose: the picture of this compose is composed after it
  const t0 = performance.now();
  const ptr = deskScene && api.scene ? api.scene() : api.compose();
  pendingComposeMs += performance.now() - t0;
  deskPtr = ptr;
  const buf = mem.buffer;   // after the compose: it can grow the memory
  const gen = ptr === 0 ? gate.published : deskScene ? sceneGen(buf, ptr) : deskGen(buf, ptr);
  const changed = ptr !== 0 && gen !== gate.published;
  let answers: number[] | undefined;
  if (changed) {
    gate.publish(gen);
    framesSent++;
    if (gate0) answers = completeInline(true);   // decision 0280: the inputs the runtime took inline are answered by the first change that shows something
    if (seqs) answers = [...(answers ?? []), ...seqs];
  }
  if (changed || reply) {
    const shared = buf instanceof SharedArrayBuffer ? buf : null;
    if (!shared && !sharedWarned) { sharedWarned = true; log('error', 'the engine memory is not shared: the render Worker cannot read the picture from it'); }
    renderPost({ t: 'frame', changed, gen: gate.published, ptr, ...(shared ? { buffer: shared } : {}), composeMs: pendingComposeMs, ringReads: deskCounters.ringReads, wakes: deskCounters.wakes, ...(answers && answers.length > 0 ? { answers } : {}) });
    pendingComposeMs = 0;
  }
  if (fruity) {
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
  }
  return changed;
}

/** the answer to a question that found the picture busy: nothing changed (the render Worker's last picture is the one it has) */
function renderNoChange(): void {
  if (port && gate) renderPost({ t: 'frame', changed: false, gen: gate.published, ptr: deskPtr, composeMs: 0, ringReads: deskCounters.ringReads, wakes: deskCounters.wakes });
}

// A test aid (any display method): the compositor is asked for the whole desktop as the window table is now, and the picture is copied out. With display method D the next present shows everything again as well.
function deskDump(peek: boolean): void {
  const api = compositor(), mem = fruity?.memory;
  if (!api || !mem) { post({ t: 'desk-dump-error', message: 'this engine build has no desktop compositor' }); return; }
  // peek: the picture as it is (what the render Worker last read), no compose; dump: the whole picture drawn again from the window table (the next present then shows everything)
  if (peek && deskScene) {   // decision 0240: with WebGL the picture 'as it is' is the scene drawn again from the textures the browser holds: the render Worker reads it back, the answer comes as 'pixels'
    pendingDump = api.describe();
    renderPost({ t: 'read-pixels' });
    return;
  }
  if (!peek && gate?.busy) { post({ t: 'desk-dump-error', message: 'the picture is being shown: ask again' }); return; }
  let st: DeskState | null = null;
  if (peek) st = deskPtr ? readDeskState(mem.buffer, deskPtr) : null;
  else { api.resync(); deskPtr = api.compose(); st = readDeskState(mem.buffer, deskPtr); }   // the buffer after compose: it can grow the memory (see DeskPresenter.present)
  if (!st) { post({ t: 'desk-dump-error', message: 'the compositor has no picture yet' }); return; }
  const data = new Uint8Array(mem.buffer, st.pixels, st.height * st.stride).slice().buffer as ArrayBuffer;   // decision 0271: a SharedArrayBuffer's slice() is a SharedArrayBuffer, which cannot be in the transfer list (DataCloneError in Firefox without WebGL); a Uint8Array's slice() copies into a plain ArrayBuffer
  if (!peek) api.resync();
  post({ t: 'desk-dump', width: st.width, height: st.height, data, windows: api.describe() }, [data]);
}
