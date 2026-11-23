// The render Worker (module worker; decision C-2). It shows the desktop picture on the one canvas the page transferred to it. The engine Worker (worker.ts) composes the picture in the engine's shared memory and says
// when it changed (EngineToRender, protocol.ts); this Worker reads the changed rectangles from that memory and draws them, with putImageData (desk.ts) or WebGL2 (desk-gl.ts). The engine's thread neither draws
// nor waits for the canvas. The engine composes only while this Worker has read the last picture (desk-link.ts), so the picture does not change while it is read.
// The frame loop is this Worker's own requestAnimationFrame, so the engine's business does not hold it back: it asks the engine to compose while the picture moves. When nothing has changed for DESK_SLEEP_AFTER_MS
// the loop sleeps and asks once every DESK_WAKE_MS; the first change that the engine reports wakes the loop.
import { sha256Hex } from '../bundle/sha256.ts';
import { DeskPresenter, type DeskApi, type DeskSurface } from './desk.ts';
import { GlDeskPresenter, webgl2Usable, type DeskShow, type GlSurface, type SceneApi } from './desk-gl.ts';
import { DeskGate } from './desk-link.ts';
import { QuietClock } from './desk-sleep.ts';
import { watchContextLoss, type ContextLossTarget } from './ctx-resync.ts';
import type { EngineToRender, FromWorker, RenderToEngine, ToRender } from './protocol.ts';

declare const self: DedicatedWorkerGlobalScope;

const TIME_ORIGIN = performance.timeOrigin;
const nowAbs = (): number => TIME_ORIGIN + performance.now();
const post = (m: FromWorker, transfer: Transferable[] = []): void => self.postMessage(m, transfer);
const log = (level: 'info' | 'warn' | 'error', text: string): void => post({ t: 'log', level, text });
const mark = (name: string, detail?: Record<string, unknown>): void => post({ t: 'mark', name, abs: nowAbs(), ...(detail ? { detail } : {}) });

/** decision 0223 / desk2: the frame loop sleeps after this long without a change (desk-sleep.ts) */
const DESK_SLEEP_AFTER_MS = 100;
/** C-2: while the picture does not move, the engine is asked this often (about 8 times a second); a change the engine makes by itself shows within this time (an input is answered at once, not by this) */
const DESK_WAKE_MS = 125;

let canvas: OffscreenCanvas | null = null;
let port: MessagePort | null = null;
let gate: DeskGate | null = null;
let presenter: DeskShow | null = null;
let memory: ArrayBufferLike | null = null;
/** the address the engine gave with the last frame: the compositor's state, or the scene in scene mode (both are the same address on every compose) */
let ptr = 0;
const engineApi: DeskApi & SceneApi = {
  compose: () => ptr,
  scene: () => ptr,
  sceneMode: (on: number) => { if (on) toEngine({ cmd: 'scene-mode' }); },
  resync: () => toEngine({ cmd: 'resync' }),
  describe: () => '',
};
const counters = { ticks: 0, idleTicks: 0, ringReads: 0, wakes: 0, engineComposeMs: 0 };
const quiet = new QuietClock(DESK_SLEEP_AFTER_MS, performance.now());
/** one question to the engine is open (the frame loop asks once per frame; the sleeping loop once per DESK_WAKE_MS) */
let waiting = false;
let loopOn = false;
let pollTimer: ReturnType<typeof setTimeout> | null = null;
let firstPixel = false;
// Canvas updates deduplicated per Worker rAF; not GPU completion or physical display evidence.
let drewSinceFrame = false;
let displayFrames: number | null = typeof self.requestAnimationFrame === 'function' ? 0 : null;
let lastReport = 0;
const presentedAt: number[] = [];
let answers: number[] = [];

function toEngine(m: RenderToEngine, transfer: Transferable[] = []): void { port?.postMessage(m, transfer); }

/** asks the engine to compose; false when a question is still open */
function ask(): boolean {
  if (waiting || !port || !gate) return false;
  waiting = true;
  gate.request();
  toEngine({ cmd: 'want' });
  return true;
}

function nextFrame(): void {
  const g = self as unknown as { requestAnimationFrame?: (cb: () => void) => number };
  if (g.requestAnimationFrame) g.requestAnimationFrame(frame); else setTimeout(frame, 16);
}

function frame(): void {
  if (!loopOn) return;
  if (drewSinceFrame) {
    if (displayFrames !== null) displayFrames++;
    drewSinceFrame = false;
  }
  if (ask()) counters.ticks++;
  if (quiet.sleepAfterFrame(performance.now())) { loopOn = false; sleepPoll(); return; }
  nextFrame();
}

function startLoop(): void {
  if (loopOn || !port) return;
  loopOn = true;
  if (pollTimer !== null) { clearTimeout(pollTimer); pollTimer = null; }
  nextFrame();
}

function sleepPoll(): void {
  if (pollTimer !== null || loopOn) return;
  pollTimer = setTimeout(poll, DESK_WAKE_MS);
}

function poll(): void {
  pollTimer = null;
  if (loopOn) return;
  if (ask()) counters.idleTicks++;
  sleepPoll();
}

function wakeDesk(): void {
  if (quiet.wake(performance.now())) startLoop();
}

self.onmessage = (ev: MessageEvent<ToRender>) => {
  const m = ev.data;
  switch (m.cmd) {
    case 'init': init(m.desk); break;
    case 'readback': void readback(m.key).catch((e: unknown) => post({ t: 'error', phase: m.cmd, message: String((e as Error)?.stack ?? e) })); break;
    case 'desk-gl-read': deskGlRead(); break;
  }
};

function init(desk: Extract<ToRender, { cmd: 'init' }>['desk']): void {
  canvas = desk.canvas;
  port = desk.port;
  gate = new DeskGate(desk.ctl);
  port.onmessage = (ev: MessageEvent<EngineToRender>) => onEngine(ev.data);
  watchContextLoss(canvas as unknown as ContextLossTarget, { lost: () => { presenter?.resync(); wakeDesk(); }, restored: () => { presenter?.resync(); wakeDesk(); } });   // a sleeping frame loop is woken: the whole picture is sent again
}

function onEngine(m: EngineToRender): void {
  if (m.t === 'desk-ready') { startDesk(m.scene); return; }
  if (m.t === 'read-pixels') { readPixelsForEngine(); return; }
  onFrame(m);
}

/** decision 0240: the description is drawn with WebGL2 when the engine has one and the canvas gives a WebGL2 context; otherwise the one picture is shown with putImageData (the choice of the Worker, once) */
function makeDeskPresenter(scene: boolean): DeskShow {
  const memFn = (): ArrayBufferLike | null => memory;
  if (scene && webgl2Usable(() => new OffscreenCanvas(1, 1) as unknown as GlSurface)) {
    try {
      const gl = new GlDeskPresenter(canvas as unknown as GlSurface, engineApi, memFn);
      const target = canvas as unknown as { addEventListener(type: string, listener: (ev: Event) => void): void };
      target.addEventListener('webglcontextlost', (ev) => { ev.preventDefault(); gl.setLost(true); });
      target.addEventListener('webglcontextrestored', () => { gl.setLost(false); wakeDesk(); });
      log('info', 'display method D: the runtime describes the desktop (windows, positions, written rectangles) and the render Worker draws it with WebGL2, one texture per window');
      return gl;
    } catch (e) {
      log('info', `display method D: no WebGL2 for the desktop canvas (${String((e as Error)?.message ?? e)}); the runtime composes one picture and the render Worker shows it with putImageData`);
    }
  } else log('info', 'display method D: no scene description in this engine build (fle_desk_scene) or no usable WebGL2 (a context with a largest texture of at least 8192); the runtime composes one picture and the render Worker shows it with putImageData');
  return new DeskPresenter(canvas as unknown as DeskSurface, engineApi, memFn);
}

function startDesk(scene: boolean): void {
  if (presenter || !canvas || !port) return;
  try { presenter = makeDeskPresenter(scene); } catch (e) { log('error', `desktop not shown: ${String((e as Error)?.stack ?? e)}`); return; }
  startLoop();
}

/** one answer of the engine: a picture that changed is shown; the engine's state is read from its memory, which nothing changes while this Worker has not answered */
function onFrame(f: Extract<EngineToRender, { t: 'frame' }>): void {
  counters.ringReads = f.ringReads;
  counters.wakes = f.wakes;
  counters.engineComposeMs += f.composeMs;
  waiting = false;
  try {
    if (f.buffer) memory = f.buffer;
    ptr = f.ptr;
    if (f.changed && presenter && memory) {
      const shown = presenter.present();
      if (shown > 0) {
        drewSinceFrame = true;
        presentedAt.push(nowAbs());
        if (!firstPixel) { firstPixel = true; mark('first_pixel', { source: 'desktop', width: canvas?.width, height: canvas?.height }); }
        if (f.answers) answers.push(...f.answers);
        if (quiet.changed(performance.now())) startLoop();
      }
    }
  } catch (e) {
    log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`);
  } finally {
    gate?.ack(f.gen);
  }
  if (answers.length > 0 && presentedAt.length > 0) flushDesk(true, answers.splice(0));
  else flushDesk(false);
}

/** the counters and the instants of the presents, at most every 100 ms (1 s when nothing was presented); `answers`: the inputs the present in this message answers */
function flushDesk(force: boolean, answerList?: number[]): void {
  if (!presenter) return;
  const now = performance.now();
  if (!force && now - lastReport < 100) return;
  if (presentedAt.length === 0 && !force && now - lastReport < 1000) return;
  lastReport = now;
  post({ t: 'desk', stats: { ...presenter.stats, displayFrames, composeMs: presenter.stats.composeMs + counters.engineComposeMs, ringReads: counters.ringReads, ringFull: 0, wakes: counters.wakes, ticks: counters.ticks, idleTicks: counters.idleTicks, sleeps: quiet.sleeps }, at: presentedAt.splice(0, 64), ...(answerList && answerList.length > 0 ? { answers: answerList } : {}), nWindows: 0 });
}

async function readback(key: string): Promise<void> {
  if (key !== 'desk' || !canvas) { post({ t: 'readback-error', key, message: `no OffscreenCanvas bound to ${key}` }); return; }
  if (presenter instanceof GlDeskPresenter) {
    const r = presenter.readPixels();
    if (!r) { post({ t: 'readback-error', key, message: 'no scene has been drawn yet' }); return; }
    post({ t: 'readback', key, width: r.width, height: r.height, sha256: await sha256Hex(r.data) });
    return;
  }
  const ctx = canvas.getContext('2d') as OffscreenCanvasRenderingContext2D;
  const d = ctx.getImageData(0, 0, canvas.width, canvas.height);
  post({ t: 'readback', key, width: canvas.width, height: canvas.height, sha256: await sha256Hex(new Uint8Array(d.data.buffer)) });
}

function deskGlRead(): void {
  if (!(presenter instanceof GlDeskPresenter)) { post({ t: 'desk-dump-error', message: 'the desktop is not shown with WebGL (the canvas gave no WebGL2 context)' }); return; }
  const r = presenter.readPixels();
  if (!r) { post({ t: 'desk-dump-error', message: 'no scene has been drawn yet' }); return; }
  const data = r.data.buffer as ArrayBuffer;
  post({ t: 'desk-gl-pixels', width: r.width, height: r.height, data }, [data]);
}

/** a test aid of the engine (desk-peek with WebGL): the scene drawn again, read back from the GL canvas */
function readPixelsForEngine(): void {
  const r = presenter instanceof GlDeskPresenter ? presenter.readPixels() : null;
  if (!r) { toEngine({ cmd: 'pixels-error', message: 'the desktop is not shown with WebGL' }); return; }
  const data = r.data.buffer as ArrayBuffer;
  toEngine({ cmd: 'pixels', width: r.width, height: r.height, data }, [data]);
}
