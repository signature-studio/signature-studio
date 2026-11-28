// The engine Worker (module worker). Owns the engine instance. The lifted FL engine runs in Module(). The runtime composes the windows into one picture of the screen (decision 0223). Since C-2 this Worker only
// composes: it says to the render Worker (render-worker.ts, over a MessagePort) that the picture changed, and that Worker draws it from the engine's shared memory (desk-link.ts is the handshake). The page gets no pixel and knows no window.
// Input arrives through a SharedArrayBuffer ring (input/ring.ts), the cursor and the title as small messages.
import { classifyEngineError, FruityEngine, type FruityHost } from './fruity.ts';
import { hostPackFromBytes } from './hostfiles.ts';
import { webView2Message } from './webview2-log.ts';
import { LazyFileSource } from './lazy-file.ts';
import { IdleDriver, newDriverStats } from './idle-driver.ts';
import { IdleTrace } from './idle-trace.ts';
import { ReportThrottle } from './report-throttle.ts';
import { TaskbarReport } from './taskbar-report.ts';
import type { EngineToRender, FromWorker, GuestTextBest, GuestTextNear, RenderToEngine, ToWorker } from './protocol.ts';
import type { CursorPictureMsg } from './cursor-picture.ts';
import type { FileDlgAnswer, FileDlgItem, FileDlgRequest } from './filedlg.ts';
import { osStateKey, osStateOf } from './os-state.ts';
import { deskGen, readDeskState, type DeskApi, type DeskState } from './desk.ts';
import { sceneGen } from './desk-gl.ts';
import { DeskGate, PRESENTER_NONE, PRESENTER_SCENE } from './desk-link.ts';
import { RingReader, takenAtSchedulerPoint } from '../input/ring.ts';
import { IN, SC } from '../input/events.ts';
import { openRegState, REG_STATE_LIMIT, type RegState } from './regstore.ts';
import { fileRecord, recentAfterOpen, recentListOf } from './recent.ts';
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
/** Issue #1341: the engine's line for the read of a real module's image (harness/two/main_two.c): the file name and whether the guest asked for it */
const IMAGE_READ = /^image-read: (.+) requested=([01]) /;
let fruity: FruityEngine | null = null;
let driver: IdleDriver | null = null;
/** Issue #1383: the saved HKCU tree of the virtual registry (regstore.ts); null when the browser keeps none */
let regState: RegState | null = null;
/** Issue #1502: FL's Recent list (the MRU values, newest first) as the journal holds it (recent.ts) */
let recentList: string[] = [];
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
let lastSentSharedBuf: SharedArrayBuffer | null = null;
/** Issue #1376: the windows painted over the WebView2 face are asked for when the picture changed or the face's bounds or visibility did; the list last sent, so that only a change is sent */
const FRONT_MAX = 64;
let frontDirty = true;
let frontSent = '';

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
    case 'desk-dump': deskDump(false); break;
    case 'desk-peek': deskDump(true); break;
    case 'guest-text': guestText(m.markers, m.radius); break;
    case 'settings-redraw': if (fruity?.redrawTree(m.lid)) driver?.wake(); break;
    case 'window-api-trace': fruity?.windowApiTrace(m.on); break;
    case 'hz': hzWanted = m.hz; ensureDriver()?.setHz(m.hz); break;   // Issue #1402: the engine is paced at the page's rate from its next idle
    case 'webview2-loaded': onWebView2Loaded(); break;
    case 'webview2-message': log('info', `the page posted a message to the engine's WebView2 (${m.json.length} characters): the engine has no WebMessageReceived model yet, so it is not delivered`); break;
    case 'clipboard-text': fruity?.answerClipRead(m.id, m.text); break;   // #1389: the engine waits for this answer (its JSPI import)
    case 'exit-request': onExitRequest(); break;
    case 'clipboard-probe-answer': fruity?.answerClipProbe(m.id, m.has); break;   // #1406: the engine waits for this answer (a question about the formats)
    case 'ping': post({ t: 'pong', id: m.id }); break;
  }
}

// Issue #1362: the page's face has loaded the document of a Navigate. The engine's page event is set, and the engine is woken if it waits: its idle fires the navigation's events (fle_wv2_pump)
function onWebView2Loaded(): void {
  if (!fruity) return;
  if (!fruity.wv2Loaded()) { log('warn', 'this engine build has no WebView2 page export (fle_g3_wv2_loaded): the navigation is not completed'); return; }
  driver?.wake();
}

async function init(m: Extract<ToWorker, { cmd: 'init' }>): Promise<void> {
  ring = new RingReader(m.desk.ring);
  port = m.desk.port;
  port.onmessage = (ev: MessageEvent<RenderToEngine>) => onRender(ev.data);
  gate = new DeskGate(m.desk.ctl);
  if (!m.assets.glue || !m.assets.dll || !m.assets.seed) throw new Error('the emscripten engine needs the glue, the seed pack and the DLL pack');
  regState = await openRegState(log);
  if (regState) mark('reg_state', { saved: regState.saved.length, bytes: regState.bytes });
  recentList = regState ? recentListOf(regState.saved) : [];
  const host: FruityHost = {
    regSaved: () => regState?.saved ?? null,
    regPut: (record) => { regState?.put(record); },
    mark,
    stdout: (line) => pushLog('out', line),
    stderr: (line) => {
      pushLog('err', line);
      const r = IMAGE_READ.exec(line);   // Issue #1341: the read of a real module's image, with the page's time (the line's own clock is the engine's)
      if (r) mark('image_read', { name: r[1], requested: r[2] === '1' });
      const w = webView2Message(line);   // Issue #1362: what the WebView2 model was asked, for the page's face of the Browser panel
      if (w) {
        if (w.op === 'bounds' || w.op === 'visible') frontDirty = true;
        post(w);
      }
    },
    log,
    idle: onEngineIdle,
    fileDlg: (id, request) => { mark('filedlg_request', { id }); post({ t: 'filedlg-request', id, request: request as FileDlgRequest }); },
    fileDlgProgress: (id, done, total) => post({ t: 'filedlg-progress', id, done, total }),
    clipWrite: (text) => post({ t: 'clipboard-write', text }),   // #1389: the engine's Copy goes to the page
    clipRead: (id) => post({ t: 'clipboard-read', id }),        // #1389: the engine's Paste asks the page for the clipboard text
    clipProbe: (id) => post({ t: 'clipboard-probe', id }),      // #1406: the engine's question about the formats asks the page whether there is text
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
// the driver's wait for a deadline is Atomics.waitAsync with a timeout on a word that only a cancel notifies (Issue #1397): a setTimeout whose nesting is deeper than five waits at least 4 ms, and the deadlines of
// the idle chain are nested that way. A cancelled wait resolves at once (the notify) and does nothing when it does.
interface DeadlineWait { live: boolean; slot: number }
type WaitAsyncResult = { async: false; value: 'not-equal' | 'timed-out' } | { async: true; value: Promise<'ok' | 'timed-out'> };
const DEADLINE_SLOTS = 16;
const deadlineWords = new Int32Array(new SharedArrayBuffer(DEADLINE_SLOTS * 4));
let nextDeadlineSlot = 0;
function deadlineWait(fn: () => void, ms: number): DeadlineWait {
  const slot = nextDeadlineSlot++ & (DEADLINE_SLOTS - 1);
  const h: DeadlineWait = { live: true, slot };
  const fire = (): void => { if (h.live) { h.live = false; fn(); } };
  const expected = Atomics.load(deadlineWords, slot);
  const w = (Atomics as unknown as { waitAsync(ia: Int32Array, i: number, v: number, t: number): WaitAsyncResult }).waitAsync(deadlineWords, slot, expected, Math.max(0, ms));
  if (w.async) void w.value.then((v) => { if (v === 'timed-out') fire(); });
  else if (w.value === 'timed-out') void Promise.resolve().then(fire);
  return h;
}
function cancelDeadlineWait(h: DeadlineWait): void {
  if (!h.live) return;
  h.live = false;
  Atomics.add(deadlineWords, h.slot, 1);
  Atomics.notify(deadlineWords, h.slot);
}

/** Issue #1402: the page's last rate of requestAnimationFrame (cmd 'hz'), kept until the driver exists to take it */
let hzWanted: number | null = null;
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
    setTimer: (fn, ms) => deadlineWait(fn, ms),
    clearTimer: (h) => cancelDeadlineWait(h as DeadlineWait),
    yieldTurn: (fn) => { bounce = fn; turns.port2.postMessage(0); },
  }, {
    processed: (list, tIdle) => {
      inlineSettledCount = 0;
      inlineSince = 0;
      flushLogs();
      const expanded: { seq: number; tPage: number; tInject: number; tIdle: number }[] = [];
      for (const x of list) {
        const extra = mergedMoveSeqs.get(x.seq);
        if (extra) {
          mergedMoveSeqs.delete(x.seq);
          for (const m of extra) expanded.push({ seq: m.seq, tPage: m.tPage, tInject: x.tInject, tIdle });
        }
        expanded.push({ seq: x.seq, tPage: x.tPage, tInject: x.tInject, tIdle });
      }
      post({ t: 'input-done', items: expanded });
      // decision 0223: what the engine drew in answer to the input is shown now when the render Worker wants a frame or after a brief quiet gap; otherwise attach seqs to the next rAF present to avoid mid-rAF duplicate uploads
      if (deskReady) {
        const seqs = expanded.map((x) => x.seq);
        try {
          if (!gate?.busy && (gate?.want || performance.now() - lastWantAt >= 50)) deskTick(false, seqs);
          else pushDeferredAnswers(seqs);
        } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
      }
    },
    dropped: (seq, reason) => {
      const extra = mergedMoveSeqs.get(seq);
      if (extra) {
        mergedMoveSeqs.delete(seq);
        for (const m of extra) post({ t: 'input-dropped', seq: m.seq, reason });
      }
      post({ t: 'input-dropped', seq, reason });
    },
    resumed: (_code, abs, why) => { if (reports.resume(why)) post({ t: 'resume', abs, why }); },
    // reported here, before the driver decides what the engine does next: the page must see this idle before the resume that may follow it at once (decision 0170: the next deadline is often already due)
    idle: (n, now, next, busyMs, at) => { reportIdle(n, now, next, busyMs, at); },
    lagCut: (ticks) => {
      lagCutTicks += ticks;
      const at = performance.now();
      if (at - lastLagLog > 5000) { lastLagLog = at; log('warn', `the virtual clock fell ${(lagCutTicks / 10000).toFixed(0)} ms further behind the wall clock than the cap allows (S-clk-maxlag); it stays behind (no virtual time is skipped)`); lagCutTicks = 0; }
    },
  }, { maxLeadMs: 1, maxLagMs: 10 });
  if (hzWanted !== null) driver.setHz(hzWanted);
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
const RING_REC = 8, INLINE_TRY_MS = 4, INLINE_GIVEUP_MS = 150, MOVE_PACE_MS = 8;
type StagedSeq = { seq: number; tPage: number };
type SchedulableInput = import('../input/events.ts').MouseRaw | import('../input/events.ts').KeyRaw | import('../input/events.ts').DisplayRaw;
let pendingMove: { ev: import('../input/events.ts').MouseRaw; seqs: StagedSeq[] } | null = null;
let lastMoveInjectAt = 0;
let moveFlushTimer: DeadlineWait | null = null;
const mergedMoveSeqs = new Map<number, StagedSeq[]>();
const inlineStaged: { ev: SchedulableInput; seqs: StagedSeq[] }[] = [];
let inlineTryAt = 0;
let inlineSince = 0;
let lastWantAt = 0;
let deferredSince = 0;
const deferredAnswerSeqs: number[] = [];
function pushDeferredAnswers(seqs: number[]): void {
  if (seqs.length === 0) return;
  const now = performance.now();
  if (deferredAnswerSeqs.length > 512 || (deferredAnswerSeqs.length > 0 && now - deferredSince >= 150)) {
    deferredAnswerSeqs.length = 0;
  }
  if (deferredAnswerSeqs.length === 0) deferredSince = now;
  deferredAnswerSeqs.push(...seqs);
}
function takeDeferredAnswers(now: number): number[] | undefined {
  if (deferredAnswerSeqs.length === 0) return undefined;
  const fresh = now - deferredSince < 150 ? deferredAnswerSeqs.slice() : undefined;
  deferredAnswerSeqs.length = 0;
  return fresh;
}
// decision 0280, L1c (PROVISIONAL): which present may answer an inline input. The runtime writes fle_input_settled() into the word after the records (fruity_idle.c): 1 = the guest has taken every event the page sent out of its queue, has retrieved every message made of
// them, and the thread that did waits in its message loop again. An input is answered only by a present that was started after a scheduler point that reported 1 (the picture then holds what the guest drew for it); a present before that, which shows
// only another change (the playhead), does not answer. No class name, no window kind.
let inlineSettledCount = 0, memI32Buf: ArrayBuffer | null = null, memI32View: Int32Array | null = null;
function ensureMemI32(mem: WebAssembly.Memory): Int32Array {
  if (memI32Buf !== mem.buffer) { memI32Buf = mem.buffer; memI32View = new Int32Array(memI32Buf); }
  return memI32View as Int32Array;
}
function readSettled(ptr: number, max: number): number {
  const mem = fruity?.memory;
  if (!mem) return 0;
  return ensureMemI32(mem)[(ptr >> 2) + max * RING_REC] ?? 0;
}
function serviceRenderWant(): void {
  if (!deskReady || !gate || !gate.want || gate.busy) return;
  const api = compositor();
  if (deskScene && api?.sceneDirty && api.sceneDirty() === 0) return;
  try { deskTick(false); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
}
function clearMoveTimer(): void {
  if (moveFlushTimer !== null) { cancelDeadlineWait(moveFlushTimer); moveFlushTimer = null; }
}
function inputPoll(ptr: number, max: number): number {
  deskPoll();   // Issue #1520: before the driver exists too (the start-up): the picture is shown at the scheduler's points
  const d = driver, mem = fruity?.memory;
  if (!ring || !d || !d.ready || !mem) return 0;
  if (d.awaitingCount > inlineSettledCount && inlineSince !== 0 && readSettled(ptr, max) === 1) {
    inlineSettledCount = d.awaitingCount;
  }
  serviceRenderWant();
  if (ring.pending === 0 && pendingMove === null) { if (d.awaitingCount > 0) answerInline(); return 0; }
  const i32 = ensureMemI32(mem), base = ptr >> 2;
  inlineStaged.length = 0;
  const stage = (ev: SchedulableInput, seqs: StagedSeq[]): void => {
    const o = base + inlineStaged.length * RING_REC;
    if (ev.k === 'mouse') { i32[o] = 1; i32[o + 1] = ev.kind; i32[o + 2] = ev.x; i32[o + 3] = ev.y; i32[o + 4] = ev.data; }
    else if (ev.k === 'key') { i32[o] = 2; i32[o + 1] = ev.kind; i32[o + 2] = ev.vk; i32[o + 3] = ev.scan; i32[o + 4] = ev.ext; }
    else { i32[o] = 3; i32[o + 1] = ev.kind; i32[o + 2] = ev.width; i32[o + 3] = ev.height; i32[o + 4] = 0; }
    i32[o + 5] = seqs[seqs.length - 1]!.seq; i32[o + 6] = 0; i32[o + 7] = 0;
    inlineStaged.push({ ev, seqs });
  };
  let drained = false;
  ring.drainN(Infinity, (r) => {
    const e = r.ev;
    if (!takenAtSchedulerPoint(e)) return false;   // syscmd and any later kind stay in the ring (with what follows them) for the IdleDriver at the next idle
    drained = true;
    if (e.k === 'mouse' && e.kind === IN.MOVE) {
      if (pendingMove === null) pendingMove = { ev: e, seqs: [{ seq: r.seq, tPage: r.tPage }] };
      else { pendingMove.ev = e; pendingMove.seqs.push({ seq: r.seq, tPage: r.tPage }); }
      return true;
    }
    const needSlots = pendingMove !== null ? 2 : 1;
    if (inlineStaged.length + needSlots > max) return false;
    if (pendingMove !== null) {
      clearMoveTimer();
      stage(pendingMove.ev, pendingMove.seqs);
      pendingMove = null;
      lastMoveInjectAt = performance.now();
    }
    stage(e, [{ seq: r.seq, tPage: r.tPage }]);
    return true;
  });
  if (pendingMove !== null && inlineStaged.length < max) {
    const now = performance.now();
    if (now - lastMoveInjectAt >= MOVE_PACE_MS) {
      clearMoveTimer();
      stage(pendingMove.ev, pendingMove.seqs);
      pendingMove = null;
      lastMoveInjectAt = now;
    }
  }
  if (drained) deskCounters.ringReads++;
  if (inlineStaged.length === 0 && d.awaitingCount > 0) answerInline();   // a record that stays in the ring (a syscmd waiting for the idle) must not stop the answers of the inputs taken before it
  return inlineStaged.length;
}
function inputDone(ptr: number, n: number): void {
  const d = driver, mem = fruity?.memory;
  if (!d || !mem) { inlineStaged.length = 0; return; }
  const i32 = ensureMemI32(mem), base = ptr >> 2, at = nowAbs();
  const done: { seq: number; tPage: number; tInject: number; ev: import('../input/events.ts').RawInput }[] = [];
  for (let i = 0; i < n; i++) {
    const st = inlineStaged[i];
    if (!st) continue;
    const accepted = i32[base + i * RING_REC + 6] === 1;
    for (const s of st.seqs) {
      if (accepted) done.push({ seq: s.seq, tPage: s.tPage, tInject: at, ev: st.ev });
      else post({ t: 'input-dropped', seq: s.seq, reason: 'the engine refused the event (its input queue is full)' });
    }
  }
  inlineStaged.length = 0;
  if (done.length > 0) { if (d.awaitingCount === 0) inlineSince = performance.now(); d.injectedInline(done); }
}
// the answer to the inputs the runtime took inline: the first compose that changed a pixel after the injection, by whichever path made it (this function, the render Worker's question: deskTick calls completeInline when it changes
// something and inputs await); this function tries a compose every INLINE_TRY_MS, and after INLINE_GIVEUP_MS without a change the inputs are finished without an answer (an input that changes no pixel)
/** the inputs the runtime took inline are done; `shown`: the present that answers them showed a change (their sequence numbers are returned, for the render Worker's latency report) */
function completeInline(shown: boolean, count?: number): number[] | undefined {
  const d = driver;
  if (!d || d.awaitingCount === 0 || inlineSince === 0) return undefined;
  const list = count !== undefined ? d.takeAwaiting(count) : d.takeAwaiting();
  if (list.length === 0) return undefined;
  const at = nowAbs();
  inlineSettledCount = Math.max(0, inlineSettledCount - list.length);
  inlineSince = d.awaitingCount > 0 ? performance.now() : 0;
  post({ t: 'input-done', items: list.map((x) => ({ seq: x.seq, tPage: x.tPage, tInject: x.tInject, tIdle: at })) });
  return shown ? list.map((x) => x.seq) : undefined;
}
function answerInline(): void {
  const d = driver;
  if (!d || d.awaitingCount === 0 || !deskReady || inlineSince === 0) return;
  const t = performance.now();
  if (inlineSettledCount === 0) { if (t - inlineSince >= INLINE_GIVEUP_MS) completeInline(false); return; }
  if (gate?.busy) return;
  if (gate?.want) {
    inlineTryAt = t;
    try { deskTick(false); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
  } else if (t - inlineTryAt >= INLINE_TRY_MS && t - lastWantAt >= 50) {
    inlineTryAt = t;
    try { deskTick(false); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
  }
  if (d.awaitingCount > 0 && performance.now() - inlineSince >= INLINE_GIVEUP_MS) completeInline(false);
}

// what the page is told about an idle (ReportThrottle: the first, then one per 40 ms; decision 0170)
function reportIdle(n: number, nowTicks: number, nextTicks: number, busyMs: number | null, abs: number): void {
  if (!reports.idle(n, abs)) return;
  const caps = { input: driver !== null, appview: fruity?.hasAppView ?? false, display: fruity?.hasDisplay ?? false };
  post({ t: 'idle', n, now: nowTicks, next: nextTicks, busyMs, abs, caps, stats: driver?.stats ?? { ...newDriverStats(), idleCount: n }, queued: driver?.queued ?? 0, ...(checkpoints.length ? { trace: checkpoints.splice(0) } : {}), memBytes: curMemory()?.buffer.byteLength, dwm: fruity?.dwmFlushes() ?? null, ...(fruity?.hostFileStats ? { hostFiles: fruity.hostFileStats } : {}) });
}

// the engine is waiting: then let the driver decide (give queued input, or wait for the engine's next deadline). The picture is not composed here (C-2): the render Worker asks for it (onRender 'want'), and its own frame loop
// is not held back by the engine's business, which is what the idle used to do with a compose every 20 ms while nothing moved
function onEngineIdle(n: number, nowTicks: number, nextTicks: number): void {
  idleAt = performance.now();
  const ck = trace.push(n, nowTicks, nextTicks);
  if (ck) checkpoints.push(ck);
  flushLogs();
  sendDeskReady();
  serviceRenderWant();
  if (saveArmed) pollSavedFiles();
  ensureDriver();
  drainRing(); // the page's input waits in the ring: it is queued now and given to the engine by onIdle below
  if (driver) driver.onIdle(n, nowTicks, nextTicks);
  else reportIdle(n, nowTicks, nextTicks, null, nowAbs());
  drainRing(); // an input that was written after the first look but before this idle was announced
  armInputWait();
}

// the page writes a record and wakes this Worker with Atomics.notify on the ring (RingWriter.notify), not with a message: one wait on the ring is pending at a time, and it is armed again
// when it returns, so a record written while the engine runs is taken at the next idle, as before
let inputWaiting = false;
function armInputWait(): void {
  if (inputWaiting || !ring || !driver) return;   // without a driver nothing takes the records: a wait for them would return at once, again and again
  inputWaiting = true;
  void ring.waitForWrite().then(() => { inputWaiting = false; deskCounters.wakes++; drainRing(); armInputWait(); });
}

// ---- decision 0193: the engine's file dialogs. The engine is suspended in Show() (JSPI) while the page asks the person; the answer arrives as a message, the library reads the File into the engine's memory and resumes it ----
let saveArmed = false;
function onFileDlgAnswer(id: number, answer: FileDlgAnswer): void {
  if (!fruity) return;
  if (answer.ok && answer.items.some((i) => i.role === 'S')) saveArmed = true;
  const taken = fruity.answerFileDlg(id, answer);
  mark('filedlg_answer', { id, taken, ok: answer.ok });
  if (!taken) log('warn', `the page answered file dialog ${id}, which the engine is not waiting for`);
  else if (answer.ok) keepOpened(answer.items);
}

/** Issue #1502: a project the person opened is kept for the next start: its content (the journal's user's file, op 5) and its place at the top of FL's Recent list (the MRU values).
 * FL's own exit does not write the list here, so the page records the open; the records go to the journal as the engine's own changes do (regstore.ts). Nothing is kept when the journal is not. */
function keepOpened(items: FileDlgItem[]): void {
  const item = items.find((i) => i.role === 'R');
  if (!regState || !item || !item.file || item.file.size === 0 || item.file.size > REG_STATE_LIMIT) return;
  const state = regState;
  const path = item.path;
  void item.file.arrayBuffer().then((buf) => {
    if (regState !== state) return;
    state.put(fileRecord(path, new Uint8Array(buf)));
    const next = recentAfterOpen(recentList, path);
    recentList = next.list;
    for (const r of next.records) state.put(r);
  }).catch((e: unknown) => log('warn', `the project is not kept for the next start: ${String((e as Error)?.message ?? e).slice(0, 200)}`));
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

/** Issue #1383: the saved registry is closed when the run ends; after an error it is dropped first (the next start begins from the seed: the saved values may be what stopped it) */
function closeRegState(dropped: boolean): void {
  if (regState && dropped) {
    regState.discard();
    log('warn', 'the engine stopped with an error: the saved registry is dropped, and the next start begins from the seed');
  }
  regState?.close();
  regState = null;
}

/** Issue #1385: the page is left (pagehide: a reload or a close). The FL main window is sent SC_CLOSE on the road the task bar's button takes (the IdleDriver, at the engine's idle), so FL
 * does what it does when the person closes its window: it closes the files it keeps open (a saved project is written at its close, and each write reaches the page's journal then).
 * Once; nothing when the engine has ended or has no such window. EXIT_SEQ is no input of the page's: the latency tracker takes no sample of it. */
const EXIT_SEQ = -1;
let runEnded = false;
let exitSent = false;
function onExitRequest(): void {
  if (exitSent || runEnded || !fruity) return;
  const lid = fruity.mainWindow();
  const d = ensureDriver();
  if (!lid || !d) { log('info', 'the page is left: the engine has no FL main window to close'); return; }
  exitSent = true;
  d.submit({ k: 'syscmd', kind: IN.SYSCMD, lid, cmd: SC.CLOSE }, EXIT_SEQ, nowAbs());
  mark('exit_request', { lid });
}

async function startFruity(f: FruityEngine): Promise<void> {
  mark('engine_start');
  const result = await f.run();
  runEnded = true;
  clearMoveTimer();
  pendingMove = null;
  closeRegState(result.kind === 'trap' || (result.kind === 'exit' && result.code !== 0));
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
function flushPendingMoveToDriver(): void {
  if (!pendingMove || !driver) return;
  clearMoveTimer();
  const pm = pendingMove;
  pendingMove = null;
  lastMoveInjectAt = performance.now();
  const last = pm.seqs[pm.seqs.length - 1]!;
  if (pm.seqs.length > 1) mergedMoveSeqs.set(last.seq, pm.seqs.slice(0, -1));
  driver.submit(pm.ev, last.seq, last.tPage);
}
function drainRing(): void {
  if (!ring) return;
  ensureDriver();
  if (!driver) return;
  const n = ring.drain((r) => {
    const e = r.ev;
    if (e.k === 'mouse' && e.kind === IN.MOVE) {
      if (pendingMove === null) pendingMove = { ev: e, seqs: [{ seq: r.seq, tPage: r.tPage }] };
      else { pendingMove.ev = e; pendingMove.seqs.push({ seq: r.seq, tPage: r.tPage }); }
      return;
    }
    flushPendingMoveToDriver();
    driver?.submit(e, r.seq, r.tPage);
  });
  if (pendingMove !== null) {
    const elapsed = performance.now() - lastMoveInjectAt;
    if (elapsed >= MOVE_PACE_MS || !driver.ready) flushPendingMoveToDriver();
    else if (moveFlushTimer === null) moveFlushTimer = deadlineWait(() => { moveFlushTimer = null; flushPendingMoveToDriver(); }, Math.max(0.5, MOVE_PACE_MS - elapsed));
  }
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
    case 'want':
      sendDeskReady();
      if (gate && !gate.want) break;
      else deskTick(true);
      break;
    case 'resync': compositor()?.resync(); break;
    case 'scene-mode': sceneOn(); break;
    case 'pixels': { const d = pendingDump; pendingDump = null; if (d !== null) post({ t: 'desk-dump', width: m.width, height: m.height, data: m.data, windows: d }, [m.data]); break; }
    case 'pixels-error': pendingDump = null; post({ t: 'desk-dump-error', message: m.message }); break;
  }
}

/** Issue #1376: the windows painted over the WebView2 face (fle_webview2_front), sent to the page when the list differs from the last one sent */
function syncFront(): void {
  const api = compositor(), mem = fruity?.memory;
  frontDirty = false;
  if (!api?.front || !mem) return;
  const p = api.front();
  const words = new Int32Array(mem.buffer, p, 1 + 4 * FRONT_MAX);
  const n = Math.min(words[0] ?? 0, FRONT_MAX);
  const rects = Array.from(words.subarray(1, 1 + 4 * n));
  const key = rects.join(',');
  if (key === frontSent) return;
  frontSent = key;
  post({ t: 'webview2', op: 'front', rects });
}

/** one compose (the picture brought up to date) and the hand-over to the render Worker: when the picture changed, it is told, with the inputs this change answers (`seqs`, and the inline ones); `reply`: the render Worker asked, so it is answered even when nothing changed. True when the picture changed */
function deskTick(reply: boolean, seqs?: number[]): boolean {
  const api = compositor(), mem = fruity?.memory;
  if (!api || !mem || !gate) { if (reply) renderNoChange(); return false; }
  if (gate.busy) { if (reply) renderNoChange(); return false; }
  const settledN = inlineSettledCount;   // decision 0280, L1c: read before the compose: the picture of this compose is composed after it
  const t0 = performance.now();
  const ptr = deskScene && api.scene ? api.scene() : api.compose();
  pendingComposeMs += performance.now() - t0;
  deskPtr = ptr;
  const buf = mem.buffer;   // after the compose: it can grow the memory
  const gen = ptr === 0 ? gate.published : deskScene ? sceneGen(buf, ptr) : deskGen(buf, ptr);
  const changed = ptr !== 0 && gen !== gate.published;
  if (changed || frontDirty) syncFront();
  let answers: number[] | undefined;
  if (changed) {
    const now = performance.now();
    gate.clearWant();
    lastWantAt = now;
    gate.publish(gen);
    framesSent++;
    if (settledN > 0) answers = completeInline(true, settledN);   // decision 0280: the inputs the runtime took inline are answered by the first change that shows something
    const deferred = takeDeferredAnswers(now);
    if (deferred) answers = answers ? [...answers, ...deferred] : deferred;
    if (seqs) answers = answers ? [...answers, ...seqs] : seqs;
  } else if (seqs) {
    pushDeferredAnswers(seqs);
  }
  if (changed || reply) {
    const shared = buf instanceof SharedArrayBuffer ? buf : null;
    if (!shared && !sharedWarned) { sharedWarned = true; log('error', 'the engine memory is not shared: the render Worker cannot read the picture from it'); }
    const sendBuf = shared !== null && shared !== lastSentSharedBuf ? shared : null;
    if (sendBuf) lastSentSharedBuf = sendBuf;
    renderPost({ t: 'frame', changed, gen: gate.published, ptr, ...(sendBuf ? { buffer: sendBuf } : {}), composeMs: pendingComposeMs, ringReads: deskCounters.ringReads, wakes: deskCounters.wakes, ...(answers && answers.length > 0 ? { answers } : {}) });
    pendingComposeMs = 0;
  }
  if (fruity) {
    const v = fruity.readAppView(lastAppSeq);
    if (v) {
      lastAppSeq = v.seq;
      const os = osStateOf(v, lastTitle);
      lastTitle = os.title;
      const key = osStateKey(os);
      if (key !== lastOs) {
        lastOs = key;
        if (os.cursorShared) {
          if (!sentSharedCursors.has(os.cursor)) sendCursorPicture(fruity, os.cursor, true);
        } else if (os.cursorMade) {
          sendCursorPicture(fruity, os.cursor, false);
        }
        post({ t: 'osstate', ...os, abs: nowAbs() });
      }
    }
    // decision 0272: a minimized window with a taskbar button, as the engine says it (null: this build has no such export; nothing is posted)
    const tb = taskbarReport.next(fruity.taskbarIconic());
    if (tb !== null) post({ t: 'taskbar', lid: tb });
  }
  return changed;
}

const sentSharedCursors = new Set<number>();
const sentMadeCursorSig = new Map<number, string>();
function cursorPicSig(p: CursorPictureMsg): string {
  const words = new Uint32Array(p.rgba);
  let h1 = 0x811c9dc5;
  let h2 = 0x9e3779b9;
  for (let i = 0; i < words.length; i++) {
    const w = words[i] as number;
    h1 = Math.imul(h1 ^ w, 0x01000193) >>> 0;
    h2 = Math.imul(h2 ^ ((w << 13) | (w >>> 19)), 0x85ebca6b) >>> 0;
  }
  return `${p.width}x${p.height}:${p.hotX},${p.hotY}:${p.alpha ? 1 : 0}:${p.invert}:${p.transparent}:${p.opaque}:${h1.toString(16)}_${h2.toString(16)}`;
}

/** decision 0240: the scene mode is on, once (the render Worker's WebGL2 presenter reads the description, not the picture) */
function sceneOn(): void {
  const api = compositor();
  if (api && api.scene && api.sceneMode && !deskScene) { api.sceneMode(1); deskScene = true; }
}

// Issue #1520: the picture is shown while the engine runs without an idle (a start-up, a long run). The render Worker's question is answered only at the engine's idle, so the scheduler's points
// (inputPoll) bring the picture up to date too: at most every DESK_POLL_MS, and only when no idle came for that long (at an idle the render Worker's own question does it). A compose waits for a presenter
// to exist (the shared block says which): the first one must show the whole picture, and nothing composed before it would be shown.
const DESK_POLL_MS = 50;
let deskPollAt = -Infinity;
let idleAt = -Infinity;
function deskPoll(): void {
  const t = performance.now();
  if (t - deskPollAt < DESK_POLL_MS || t - idleAt < DESK_POLL_MS) return;
  deskPollAt = t;
  if (!port || !gate || !fruity) return;
  sendDeskReady();
  const mode = gate.presenter;
  if (!deskReady || mode === PRESENTER_NONE) return;
  if (mode === PRESENTER_SCENE) sceneOn();
  try { deskTick(false); } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
}

/** #1392: the picture of a cursor the guest loaded from one of its modules or made with CreateIconIndirect, read now (before the osstate that names the handle, which is posted when the key changes: a handle number can be made again for another cursor, so it is not kept) */
function sendCursorPicture(f: FruityEngine, handle: number, shared: boolean): void {
  const p = f.cursorPicture(handle);
  if (p === null) { post({ t: 'cursorpic', handle, why: 'this engine build has no cursor picture export', picture: null }); return; }
  if (!p.ok || p.picture === null) { post({ t: 'cursorpic', handle, why: p.why, picture: null }); return; }
  if (shared) {
    sentSharedCursors.add(handle);
  } else {
    const sig = cursorPicSig(p.picture);
    if (sentMadeCursorSig.get(handle) === sig) return;
    if (sentMadeCursorSig.size >= 256) sentMadeCursorSig.clear();
    sentMadeCursorSig.set(handle, sig);
  }
  post({ t: 'cursorpic', handle, why: '', picture: p.picture }, p.picture.raw ? [p.picture.rgba, p.picture.raw] : [p.picture.rgba]);
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

// #1358 (a test aid, any display method): the text the guest's memory holds for each marker. Read only. A marker is searched as ASCII and as UTF-16LE.
// Each place it is found at gives the printable run around it (forward and backward, TEXT_REACH bytes at most); the text with the most lines is the answer.
// The markers are tried in order, and the search stops at the first text of three lines or more. radius > 0 also lists the printable runs near every place.
const TEXT_REACH = 0x100000;
const TEXT_HITS = 64;
const TEXT_LINES = 2000;
const TEXT_LINE_MAX = 400;
const NEAR_PLACES = 400;
const NEAR_RUNS = 200;

const textUnit = (wide: boolean): number => (wide ? 2 : 1);
const textChar = (b: Uint8Array, p: number, wide: boolean): number => (wide ? (b[p] ?? 0) | ((b[p + 1] ?? 0) << 8) : (b[p] ?? 0));
const isText = (c: number): boolean => (c >= 0x20 && c <= 0x7e) || c === 0x09 || c === 0x0a || c === 0x0d;

/** the places (byte offsets) of `s` in the memory, as ASCII or as UTF-16LE, at most `limit`: the first byte is found by indexOf and the rest compared */
function findPlaces(b: Uint8Array, s: string, wide: boolean, limit: number): number[] {
  const u = textUnit(wide);
  const pat = new Uint8Array(s.length * u);
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    pat[i * u] = c & 0xff;
    if (wide) pat[i * u + 1] = c >> 8;
  }
  const out: number[] = [];
  const last = b.length - pat.length;
  const first = pat[0] ?? 0;
  let p = b.indexOf(first, 0);
  while (p >= 0 && p <= last && out.length < limit) {
    let k = 1;
    while (k < pat.length && b[p + k] === pat[k]) k++;
    if (k === pat.length) out.push(p);
    p = b.indexOf(first, p + 1);
  }
  return out;
}

/** the printable run [start, end) through the place [at, at + len), at most TEXT_REACH bytes each way */
function textBounds(b: Uint8Array, at: number, len: number, wide: boolean): [number, number] {
  const u = textUnit(wide);
  let s = at, e = at + len;
  while (at - s < TEXT_REACH && s - u >= 0 && isText(textChar(b, s - u, wide))) s -= u;
  while (e - at < TEXT_REACH && e + u <= b.length && isText(textChar(b, e, wide))) e += u;
  return [s, e];
}

function decodeText(b: Uint8Array, s: number, e: number, wide: boolean): string {
  let out = '';
  for (let p = s; p < e; p += textUnit(wide)) out += String.fromCharCode(textChar(b, p, wide));
  return out;
}

/** the printable runs (two characters and more) within `radius` bytes of the place `at` */
function runsNear(b: Uint8Array, at: number, radius: number, wide: boolean): string[] {
  const u = textUnit(wide);
  const out: string[] = [];
  let cur = '';
  const to = Math.min(at + radius, b.length - u);
  for (let p = Math.max(0, at - radius); p <= to; p += u) {
    const c = textChar(b, p, wide);
    if (c >= 0x20 && c <= 0x7e) cur += String.fromCharCode(c);
    else {
      if (cur.length >= 2 && out.length < NEAR_RUNS) out.push(cur.slice(0, TEXT_LINE_MAX));
      cur = '';
    }
  }
  if (cur.length >= 2 && out.length < NEAR_RUNS) out.push(cur.slice(0, TEXT_LINE_MAX));
  return out;
}

function guestText(markers: string[], radius: number): void {
  const mem = fruity?.memory;
  if (!mem) { post({ t: 'desk-dump-error', message: 'the engine has no memory to read yet' }); return; }
  const b = new Uint8Array(mem.buffer);
  let best = null as GuestTextBest | null;
  const near: GuestTextNear[] = [];
  for (const marker of markers) {
    for (const wide of [false, true]) {
      const enc = wide ? 'utf16le' : 'ascii';
      // #1391: the runs near a place are listed for up to NEAR_PLACES places per encoding (the top bar's "<n> MB" is not cut off by the lowest addresses); the best text still comes from the first TEXT_HITS places
      const places = findPlaces(b, marker, wide, radius > 0 ? NEAR_PLACES : TEXT_HITS);
      for (const [i, at] of places.entries()) {
        if (i < TEXT_HITS) {
          const [s, e] = textBounds(b, at, marker.length * textUnit(wide), wide);
          const lines = decodeText(b, s, e, wide).split(/\r\n|\r|\n/).filter((l) => l.length > 0);
          if (best === null || lines.length > best.lineCount) best = { marker, enc, offset: at, lineCount: lines.length, lines: lines.slice(0, TEXT_LINES).map((l) => l.slice(0, TEXT_LINE_MAX)) };
        }
        if (radius > 0) near.push({ marker, enc, offset: at, runs: runsNear(b, at, radius, wide) });
      }
    }
    if (best !== null && best.lineCount >= 3) break;
  }
  post({ t: 'guest-text', memBytes: mem.buffer.byteLength, best, near });
}
