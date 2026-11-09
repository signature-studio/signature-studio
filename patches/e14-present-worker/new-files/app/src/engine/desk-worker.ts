// Decision 0311 (gpu1, PROVISIONAL, measurement tree): the present Worker `P`. It owns the desktop's OffscreenCanvas and WebGL2 context (the engine Worker hands them over when the engine has a scene description and the platform
// gives WebGL2: worker.ts), keeps the frame loop (requestAnimationFrame of this Worker, delivered at once because nothing else runs here) and draws what the engine Worker publishes through the link (desk-link.ts) with
// GlDeskPresenter, the same code that ran in the engine Worker before, reading the surfaces' pixels from the engine's own linear memory (texSubImage2D straight from it; no pixel is posted anywhere).
// It decides nothing about windows: it draws the description it is given.
import { GlDeskPresenter, webgl2Usable, type GlSurface, type SceneApi } from './desk-gl.ts';
import { DeskLinkP } from './desk-link.ts';
import { QuietClock } from './desk-sleep.ts';
import { watchContextLoss, type ContextLossTarget } from './ctx-resync.ts';
import { sha256Hex } from '../bundle/sha256.ts';
import type { FromWorker } from './protocol.ts';

declare const self: DedicatedWorkerGlobalScope;

const TIME_ORIGIN = performance.timeOrigin;
const nowAbs = (): number => TIME_ORIGIN + performance.now();
const post = (m: FromWorker, transfer: Transferable[] = []): void => self.postMessage(m, transfer);
const log = (level: 'info' | 'warn' | 'error', text: string): void => post({ t: 'log', level, text });
/** when no present changed a pixel for this long the frame loop sleeps (the engine's idle finds the next change); the same value as the engine Worker had */
const DESK_SLEEP_AFTER_MS = 100;

let link: DeskLinkP | null = null;
let port: MessagePort | null = null;
let pres: GlDeskPresenter | null = null;
let canvas: OffscreenCanvas | null = null;
const quiet = new QuietClock(DESK_SLEEP_AFTER_MS, performance.now());
let nextFrame: (() => void) | null = null;
let frameTicks = 0;
const presentedAt: number[] = [];
let lastReport = 0;

self.onmessage = (ev: MessageEvent<{ cmd: 'init'; link: SharedArrayBuffer; port: MessagePort }>) => {
  const m = ev.data;
  if (m.cmd !== 'init') return;
  link = new DeskLinkP(m.link);
  port = m.port;
  // the platform decides, before the canvas is handed over: this Worker can wait on the link (Atomics.waitAsync) and gives a WebGL2 context without a major performance caveat; else the engine Worker keeps the desktop as it did
  link.setReady(typeof (Atomics as unknown as { waitAsync?: unknown }).waitAsync === 'function' && webgl2Usable(() => new OffscreenCanvas(1, 1) as unknown as GlSurface) ? 3 : 2);
  port.onmessage = (e: MessageEvent<{ cmd: 'handover'; canvas: OffscreenCanvas; memory: WebAssembly.Memory } | { cmd: 'readback' | 'desk-gl-read' | 'desk-peek'; key?: string; windows?: string }>) => {
    const d = e.data;
    if (d.cmd === 'handover') start(d.canvas, d.memory);
    else if (d.cmd === 'readback') void readback(d.key ?? 'desk');
    else if (d.cmd === 'desk-gl-read') glRead('desk-gl-pixels', '');
    else if (d.cmd === 'desk-peek') glRead('desk-dump', d.windows ?? '');
  };
};

function start(c: OffscreenCanvas, memory: WebAssembly.Memory): void {
  const L = link as DeskLinkP;
  canvas = c;
  try {
    const api: SceneApi = {
      scene: () => L.slotOffset,
      sceneMode: (on) => L.requestMode(on),
      resync: () => L.requestResync(),
      describe: () => '',
      deferred: true,
    };
    pres = new GlDeskPresenter(c as unknown as GlSurface, api, () => memory.buffer, () => performance.now(), () => L.buffer);
    const target = c as unknown as ContextLossTarget & { addEventListener(type: string, listener: (ev: Event) => void): void };
    target.addEventListener('webglcontextlost', (ev) => { ev.preventDefault(); pres?.setLost(true); });
    target.addEventListener('webglcontextrestored', () => { pres?.setLost(false); wake(); });
    watchContextLoss(target, { lost: () => { pres?.resync(); wake(); }, restored: () => { pres?.resync(); wake(); } });
  } catch (e) {
    L.setReady(2);
    log('error', `present Worker: ${String((e as Error)?.stack ?? e)}`);
    return;
  }
  log('info', 'display method D (present Worker): the engine Worker publishes the description of the desktop (windows, positions, written rectangles); this Worker draws it with WebGL2, one texture per window, from the engine\'s memory');
  L.setReady(1);
  startFrameLoop();
  void present();
}

function wake(): void { if (quiet.wake(performance.now())) { link?.setAsleep(false); nextFrame?.(); } }

function startFrameLoop(): void {
  const L = link as DeskLinkP;
  const g = self as unknown as { requestAnimationFrame?: (cb: (t: number) => void) => number };
  const next = (): void => { if (g.requestAnimationFrame) g.requestAnimationFrame(tick); else setTimeout(tick, 16); };
  const tick = (): void => {
    frameTicks++;
    const now = performance.now();
    L.requestScene();
    if (quiet.sleepAfterFrame(now)) { L.setAsleep(true); return; }
    next();
  };
  nextFrame = next;
  quiet.wake(performance.now());
  next();
}

/** draws each scene the engine Worker publishes, one at a time */
async function present(): Promise<void> {
  const L = link as DeskLinkP;
  let seen = L.done;
  for (;;) {
    const seq = await L.waitPublished(seen);
    seen = seq;
    let shown = 0;
    try { shown = pres?.present() ?? 0; } catch (e) { log('error', `desktop present failed: ${String((e as Error)?.stack ?? e)}`); }
    const at = nowAbs();
    if (shown > 0) {
      presentedAt.push(at);
      if (quiet.changed(performance.now())) { L.setAsleep(false); nextFrame?.(); }
    }
    L.markDone(seq, shown, at);
    report(false);
  }
}

function report(force: boolean): void {
  const L = link as DeskLinkP, now = performance.now();
  if (!pres) return;
  if (!force && now - lastReport < 100) return;
  if (presentedAt.length === 0 && !force && now - lastReport < 1000) return;
  lastReport = now;
  post({ t: 'desk', stats: { ...pres.stats, ringReads: L.counter(10), ringFull: 0, wakes: L.counter(11), ticks: frameTicks, idleTicks: L.counter(12), sleeps: quiet.sleeps }, at: presentedAt.splice(0, 64), nWindows: 0 });
}

function glRead(kind: 'desk-gl-pixels' | 'desk-dump', windows: string): void {
  const r = pres?.readPixels();
  if (!r) { post({ t: 'desk-dump-error', message: pres ? 'no scene has been drawn yet' : 'the desktop is not shown with WebGL (the canvas gave no WebGL2 context)' }); return; }
  const data = r.data.buffer as ArrayBuffer;
  if (kind === 'desk-gl-pixels') post({ t: 'desk-gl-pixels', width: r.width, height: r.height, data }, [data]);
  else post({ t: 'desk-dump', width: r.width, height: r.height, data, windows }, [data]);
}

async function readback(key: string): Promise<void> {
  if (key !== 'desk' || !canvas) { post({ t: 'readback-error', key, message: `no OffscreenCanvas bound to ${key}` }); return; }
  const r = pres?.readPixels();
  if (!r) { post({ t: 'readback-error', key, message: 'no scene has been drawn yet' }); return; }
  post({ t: 'readback', key, width: r.width, height: r.height, sha256: await sha256Hex(r.data) });
}
