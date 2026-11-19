// The measurement beacon (decision 0152): what the page sends to POST /__perf of its own origin, when, and how an answer is read.
//   sent: once when the real engine reaches its first idle ('first-idle'), then every 30 s while it runs ('periodic', at most 20), and once when it ends ('exit' or 'error': the first of the two).
//   Nothing else: no other origin (the CSP says connect-src 'self'), no cookie of the app's own (credentials: 'same-origin': the only cookie that can go with it is Cloudflare Access's CF_Authorization, which Access needs to let the POST through to the Worker; the Worker keeps neither it nor the identity, worker/perf-store.ts), no IP address, no field input.
import type { BrowserInfo, ClientInfo } from './browser-info.ts';
import type { DisplaySnapshot, HzInfo } from './display.ts';
import { TARGETS, WASM_MEMORY_LIMIT_BYTES, type InputLatencyStats, type IntervalStats } from './stats.ts';
import { PERF_PATH, PERF_MAX_BODY, PERF_STORED_HEADER } from '../../shared/perf.ts';
import { sanitizeEngineText } from './exit-message.ts';
import type { GuestScreenInfo } from '../ui/screen-fit.ts';
import type { FilesUsage } from '../files/usage.ts';
import type { AssetsReport } from '../bundle/assets-report.ts';

export type BeaconReason = 'first-idle' | 'periodic' | 'exit' | 'error';
export const BEACON_PERIOD_MS = 30_000;
export const BEACON_MAX_PERIODIC = 20;
/**
 * 2 (decision 0192): adds `client`, `display` and `hz`; every field of version 1 is unchanged.
 * 3 (decision 0202): `input` measures from the DOM event's own timeStamp to the next animation frame after the answer was drawn (`input.eventToRaf`, the P2 number) and keeps version 2's definition under `input.listenerToPut`; the
 * flat `input.n / p50Ms / p95Ms / maxMs / lastMs` of versions 1 and 2 (listener -> putImageData) are gone from `input`, so that nothing reads the old definition as the new one. `input.stages` says where the time goes.
 * Still 3 (decision 0209): `engine.exit.message` is an OPTIONAL string added to the existing `engine.exit` object (the engine's last words, at most 400 characters, personal parts removed: exit-message.ts). A version 3 reader that
 * does not know it ignores it (the server stores the object as it is; shared/perf.ts beaconProblems checks the field only when it is there), and a record without it is a record of the engine before this change. * Decision 0205 (still version 3: it has not been deployed): the answer is the frame posted in the processing window; `input.answered / late / noAnswer` count the processed inputs by how they were answered, and an input with no answer is not in the percentiles.
 */
export const BEACON_VERSION = 3;

export interface BeaconPayload {
  v: typeof BEACON_VERSION;
  seq: number;
  /** decision 0192: a random token made when this page was loaded and sent with every beacon of it (never stored in the browser): the log can tell which records are one page load. Not an identifier of a person or a machine. */
  sid: string;
  reason: BeaconReason;
  /** performance.now() of the page when the beacon was made */
  pageMs: number;
  visibility: string | null;
  browser: BrowserInfo;
  bundle: { name: string | null; version: string | null; engineKind: string | null };
  engine: { present: 'D'; updatesObservable: true; state: string; exit: { reason: string; code: number; presents: number; /** decision 0209: the engine's last words (at most 400 characters, no paths or names); absent: none printed, or a record from before this field */ message?: string } | null };
  error: { what: string; where: string } | null;
  /** instants of the start-up in ms since the navigation started (window.__app.timeline) */
  timeline: Record<string, number>;
  startup: { firstIdleSeconds: number | null; targetSeconds: number };
  ui: { updates: { total: number; recent10s: IntervalStats } | null; engineIdles: { total: number; recent10s: IntervalStats }; targetUpdatesPerSecond: number };
  input: InputLatencyStats & { targetP95Ms: number };
  /** AudioWorklet under-runs; null: nothing is measured (no AudioWorklet, or the audio output has not run yet) */
  audio: { underruns: number | null; /** decision 0175/0211: present when the page has an audio output (everywhere but a browser without AudioWorklet): status, ctxState, sampleRate, baseLatencyMs, outputLatencyMs, autoplayPolicy, gestures, lastResume, fillFrames, underruns ... (PageAudio.detail()) */ detail?: Record<string, unknown> };
  /** decision 0170: the size of the engine's wasm memory at the last idle report, the limit of its memory section, and the fraction (null: no engine memory yet) */
  memory: { wasmBytes: number | null; wasmLimitBytes: number; wasmFraction: number | null };
  /** decision 0192: the browser/OS name, viewport, colour depth, how the page was reached, and why there is no physical core count; null when the page did not collect it */
  client: ClientInfo | null;
  /** decision 0192: what the page saw of the screen — requestAnimationFrame, canvas updates, long tasks, slow input events, visibility; null when the display meter was not running (with the panel never opened) */
  display: DisplaySnapshot | null;
  /** decision 0192: the engine's rate against the screen's rAF rate; null when the display meter was not running */
  hz: HzInfo | null;
  /** decision 0212 (additive, still version 3): which display the page asked for and what the engine uses (fit / fit-device / fixed / legacy, the guest's screen, the DPI told, the room, the device pixel ratio); null when the page has not decided yet */
  guestScreen: GuestScreenInfo | null;
  /** decision 0216: what the page counted about its file manager and the engine's file dialogs (counts and failure codes only, no names); absent until Files was used. Still beacon version 3: optional block, like engine.exit.message */
  files?: FilesUsage;
  /** decision 0220: where the four start-up assets came from (network / HTTP cache / the page's own stored copy) and where the time went; absent: a page from before this change. Still beacon version 3: optional block, like `files` */
  assets?: AssetsReport;
  /** decision 0240: what the desktop presenter did - the way the desktop is shown ('gl': WebGL2 from the runtime's scene description, '2d': putImageData of the composed picture), the Worker's counters since the page loaded and since the previous beacon (the previous one that was SENT); absent: the stand-in or a page from before this change. Still beacon version 3: optional block, like `assets` */
  desk?: DeskBeacon;
  /** decision 0240: how the engine itself ran in the last (at most) 10 s before this beacon, from its idle reports: the share of the wall time it was running (busyFraction = sum of the 'ran since it was last resumed' times / the wall time), those times' median and p95, and the virtual clock's advance per wall ms (1: real time; below 1: the guest cannot keep up); absent: fewer than 2 idles in the window */
  engineRun?: EngineRunBeacon;
}

export interface DeskCounters { presents: number; rects: number; mpx: number; composeMs: number; putMs: number; drawMs: number; uploads: number; draws: number; whole: number; ticks: number; idleTicks: number; wakes: number; sleeps: number }
export interface DeskBeacon extends DeskCounters { mode: 'gl' | '2d'; since: (DeskCounters & { seconds: number }) | null }
export interface EngineRunBeacon { windowSeconds: number; idles: number; busyFraction: number; busyMsMedian: number; busyMsP95: number; virtualPerWall: number }

/** the presenter's counters as the Worker reports them (engine/protocol.ts 'desk' message; the GL presenter adds drawMs, uploads, draws) in the beacon's units */
export function deskCounters(s: Record<string, unknown> | null | undefined): DeskCounters | null {
  if (!s) return null;
  const n = (k: string): number => (typeof s[k] === 'number' && Number.isFinite(s[k] as number) ? (s[k] as number) : 0);
  const r = (x: number, d: number): number => Math.round(x * 10 ** d) / 10 ** d;
  return { presents: n('presents'), rects: n('rects'), mpx: r(n('pixels') / 1e6, 3), composeMs: r(n('composeMs'), 1), putMs: r(n('putMs'), 1), drawMs: r(n('drawMs'), 1), uploads: n('uploads'), draws: n('draws'), whole: n('whole'), ticks: n('ticks'), idleTicks: n('idleTicks'), wakes: n('wakes'), sleeps: n('sleeps') };
}
export function deskBeacon(now: Record<string, unknown> | null | undefined, prev: { stats: Record<string, unknown>; atMs: number } | null, atMs: number): DeskBeacon | null {
  const c = deskCounters(now);
  if (!c) return null;
  const p = prev ? deskCounters(prev.stats) : null;
  const since = p && prev ? { seconds: Math.round((atMs - prev.atMs) / 100) / 10, presents: c.presents - p.presents, rects: c.rects - p.rects, mpx: Math.round((c.mpx - p.mpx) * 1000) / 1000, composeMs: Math.round((c.composeMs - p.composeMs) * 10) / 10, putMs: Math.round((c.putMs - p.putMs) * 10) / 10, drawMs: Math.round((c.drawMs - p.drawMs) * 10) / 10, uploads: c.uploads - p.uploads, draws: c.draws - p.draws, whole: c.whole - p.whole, ticks: c.ticks - p.ticks, idleTicks: c.idleTicks - p.idleTicks, wakes: c.wakes - p.wakes, sleeps: c.sleeps - p.sleeps } : null;
  return { mode: typeof (now as Record<string, unknown>).draws === 'number' ? 'gl' : '2d', ...c, since };
}
/** the idle reports of the last `windowMs` before `atMs` (wall ms, performance.timeOrigin based like `at`); `now` is the virtual clock in 100 ns ticks (10,000 per ms) */
export function engineRunBeacon(idles: readonly { at: number; busyMs: number | null; now: number }[], atMs: number, windowMs = 10_000): EngineRunBeacon | null {
  const w = idles.filter((x) => x.at >= atMs - windowMs && x.at <= atMs);
  if (w.length < 2) return null;
  const first = w[0] as { at: number; now: number }, last = w[w.length - 1] as { at: number; now: number };
  const wall = last.at - first.at;
  if (!(wall > 0)) return null;
  const busy = w.slice(1).map((x) => x.busyMs).filter((x): x is number => typeof x === 'number' && x >= 0).sort((a, b) => a - b);
  const sum = busy.reduce((a, b) => a + b, 0);
  const q = (f: number): number => (busy.length ? (busy[Math.min(busy.length - 1, Math.floor(f * busy.length))] as number) : 0);
  return { windowSeconds: Math.round(wall / 100) / 10, idles: w.length, busyFraction: Math.round((sum / wall) * 1000) / 1000, busyMsMedian: Math.round(q(0.5) * 100) / 100, busyMsP95: Math.round(q(0.95) * 100) / 100, virtualPerWall: Math.round(((last.now - first.now) / 10_000 / wall) * 1000) / 1000 };
}

export interface BeaconInput {
  seq: number; sid?: string; reason: BeaconReason; pageMs: number; visibility: string | null; browser: BrowserInfo;
  bundle: { name: string | null; version: string | null; engineKind: string | null };
  state: string;
  exit: { reason: string; code: number; presents: number; message?: string | null } | null;
  error: { what: string; where: string } | null;
  timeline: Record<string, number>;
  updates: { total: number; recent: IntervalStats };
  idles: { total: number; recent: IntervalStats };
  input: InputLatencyStats;
  underruns: number | null;
  /** decision 0175/0211: the audio output's detail (PageAudio.detail()); absent without an audio output */
  audioDetail?: Record<string, unknown> | null;
  /** the engine's wasm memory in bytes as of the last idle report (window.__app.engineMemoryBytes); absent or null before the first idle */
  wasmMemoryBytes?: number | null;
  /** decision 0192 */
  client?: ClientInfo | null;
  display?: DisplaySnapshot | null;
  hz?: HzInfo | null;
  /** decision 0212 */
  guestScreen?: GuestScreenInfo | null;
  /** decision 0216 */
  files?: FilesUsage | null;
  /** decision 0220 */
  assets?: AssetsReport | null;
  /** decision 0240 */
  desk?: DeskBeacon | null;
  engineRun?: EngineRunBeacon | null;
}

/** the `engine.exit` object: the message goes through the same cleaning as when the page chose it (idempotent), so that no caller can send a path or a name by passing a raw string */
function exitBlock(e: NonNullable<BeaconInput['exit']>): NonNullable<BeaconPayload['engine']['exit']> {
  const message = typeof e.message === 'string' ? sanitizeEngineText(e.message) : '';
  return { reason: e.reason, code: e.code, presents: e.presents, ...(message !== '' ? { message } : {}) };
}

export function buildBeacon(i: BeaconInput): BeaconPayload {
  const idleMs = i.timeline.idle;
  return {
    v: BEACON_VERSION, seq: i.seq, sid: i.sid ?? '', reason: i.reason, pageMs: i.pageMs, visibility: i.visibility,
    browser: i.browser,
    bundle: i.bundle,
    engine: { present: 'D', updatesObservable: true, state: i.state, exit: i.exit ? exitBlock(i.exit) : null },
    error: i.error ? { what: i.error.what.slice(0, 400), where: i.error.where.slice(0, 200) } : null,
    timeline: i.timeline,
    startup: { firstIdleSeconds: typeof idleMs === 'number' ? idleMs / 1000 : null, targetSeconds: TARGETS.startupSeconds },
    ui: { updates: { total: i.updates.total, recent10s: i.updates.recent }, engineIdles: { total: i.idles.total, recent10s: i.idles.recent }, targetUpdatesPerSecond: TARGETS.updatesPerSecond },
    input: { ...i.input, targetP95Ms: TARGETS.inputToUpdateP95Ms },
    audio: { underruns: i.underruns, ...(i.audioDetail ? { detail: i.audioDetail } : {}) },
    memory: { wasmBytes: i.wasmMemoryBytes ?? null, wasmLimitBytes: WASM_MEMORY_LIMIT_BYTES, wasmFraction: i.wasmMemoryBytes ? i.wasmMemoryBytes / WASM_MEMORY_LIMIT_BYTES : null },
    client: i.client ?? null,
    display: i.display ?? null,
    hz: i.hz ?? null,
    guestScreen: i.guestScreen ?? null,
    ...(i.files ? { files: i.files } : {}),
    ...(i.assets ? { assets: i.assets } : {}),
    ...(i.desk ? { desk: i.desk } : {}),
    ...(i.engineRun ? { engineRun: i.engineRun } : {}),
  };
}

/** the JSON text of a beacon, under the server's limit: long strings are cut first, then the measurement of the screen is thinned out (its histograms and the second windows), and as a last resort the timeline's details and the user agent are dropped */
export function beaconText(p: BeaconPayload, max = PERF_MAX_BODY - 512): string {
  let s = JSON.stringify(p);
  if (s.length <= max) return s;
  s = JSON.stringify({ ...p, browser: { ...p.browser, userAgent: p.browser.userAgent.slice(0, 80), webglRenderer: p.browser.webglRenderer?.slice(0, 60) ?? null }, error: p.error ? { what: p.error.what.slice(0, 100), where: p.error.where.slice(0, 60) } : null });
  if (s.length <= max) return s;
  const thin = p.display ? { ...p.display, raf: { ...p.display.raf, sinceIdle: null, total: { ...p.display.raf.total, bins: [] }, recent10s: { ...p.display.raf.recent10s, bins: [] } }, canvas: p.display.canvas ? { ...p.display.canvas, sinceIdle: null } : null } : null;
  s = JSON.stringify({ ...p, display: thin, browser: { ...p.browser, userAgent: p.browser.userAgent.slice(0, 80), webglRenderer: p.browser.webglRenderer?.slice(0, 60) ?? null }, error: p.error ? { what: p.error.what.slice(0, 100), where: p.error.where.slice(0, 60) } : null });
  if (s.length <= max) return s;
  return JSON.stringify({ ...p, display: thin, timeline: {}, input: { ...p.input, eventToRaf: { ...p.input.eventToRaf, lastMs: p.input.eventToRaf.lastMs.slice(-5) }, listenerToPut: { ...p.input.listenerToPut, lastMs: p.input.listenerToPut.lastMs.slice(-5) }, lateVisible: p.input.lateVisible ? { ...p.input.lateVisible, lastMs: p.input.lateVisible.lastMs.slice(-5) } : null, eventToRafWithLate: p.input.eventToRafWithLate ? { ...p.input.eventToRafWithLate, lastMs: p.input.eventToRafWithLate.lastMs.slice(-5) } : null }, error: null });
}

export interface SendResult { ok: boolean; status: number | null; stored: boolean | null; note: string }

/** POST the beacon to this origin. Never throws: a server that refuses it, or none, is a result. */
export async function postBeacon(text: string, fetchImpl: typeof fetch = fetch): Promise<SendResult> {
  try {
    const res = await fetchImpl(PERF_PATH, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: text, credentials: 'same-origin', cache: 'no-store', keepalive: text.length < 60_000 });
    const stored = res.headers.get(PERF_STORED_HEADER);
    return { ok: res.ok, status: res.status, stored: stored === 'yes' ? true : stored === 'no' ? false : null, note: res.ok ? (stored === 'yes' ? 'stored by the server' : 'accepted, not stored by this server') : `HTTP ${res.status}` };
  } catch (e) { return { ok: false, status: null, stored: null, note: `request failed: ${String((e as Error)?.message ?? e)}` }; }
}

export interface SchedulerDeps {
  send(reason: BeaconReason): void;
  setTimer(fn: () => void, ms: number): unknown;
  clearTimer(handle: unknown): void;
  periodMs?: number;
  maxPeriodic?: number;
}

/** when beacons go out: the first at the first idle, then every period while the engine runs (up to maxPeriodic), and one when it ends */
export class BeaconScheduler {
  private handle: unknown = null;
  private firstSent = false;
  private terminalSent = false;
  private periodic = 0;
  private readonly period: number;
  private readonly maxPeriodic: number;
  private readonly deps: SchedulerDeps;
  constructor(deps: SchedulerDeps) {
    this.deps = deps;
    this.period = deps.periodMs ?? BEACON_PERIOD_MS;
    this.maxPeriodic = deps.maxPeriodic ?? BEACON_MAX_PERIODIC;
  }
  get sentPeriodic(): number { return this.periodic; }
  get running(): boolean { return this.handle !== null; }

  firstIdle(): void {
    if (this.firstSent || this.terminalSent) return;
    this.firstSent = true;
    this.deps.send('first-idle');
    this.arm();
  }

  /** the engine ended or the page failed: one last beacon (the first of 'exit' / 'error'), and no more periodic ones */
  end(reason: 'exit' | 'error'): void {
    this.stop();
    if (this.terminalSent) return;
    this.terminalSent = true;
    this.firstSent = true;
    this.deps.send(reason);
  }

  stop(): void { if (this.handle !== null) { this.deps.clearTimer(this.handle); this.handle = null; } }

  private arm(): void {
    if (this.periodic >= this.maxPeriodic) return;
    this.handle = this.deps.setTimer(() => {
      this.handle = null;
      if (this.terminalSent || this.periodic >= this.maxPeriodic) return;
      this.periodic++;
      this.deps.send('periodic');
      this.arm();
    }, this.period);
  }
}
