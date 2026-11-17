// What the page can see of the SCREEN it runs on (decision 0192): the rate of requestAnimationFrame, whether the canvases were really updated in each of those frames, how long the page took to draw what the Worker posted,
// long tasks, slow input events, how long the page was visible. Pure meters (no DOM, no network: the times are handed in) and a small monitor that wires them to the browser through a `DisplayHost` (test/unit/perf-display.test.ts
// drives it with a fake). Nothing here touches the engine, the Worker or a canvas: it only reads clocks and counts.
// Definitions (all times in ms on one clock, performance.timeOrigin + performance.now(), the clock of PerfRecorder):
//   tick            one requestAnimationFrame callback while the page is visible (`t` is the callback's DOMHighResTimeStamp + timeOrigin)
//   interval        the time between two consecutive ticks; an interval that spans a hidden period, or does not start at a tick of this visible stretch, is NOT an interval (rAF stops in a hidden tab: that is not a slow frame)
//   rAF Hz          1000 / the median interval ("achieved"); peak Hz = 1000 / the 10th percentile interval (the fastest cluster = the display's own rate when the page keeps up for at least a tenth of the frames) — S-cbeacon1-peak
//   slow frame      an interval longer than 1.5 x the window's median interval (at least one vsync missed) — S-cbeacon1-slow; "over 12.5 ms" = longer than 1.5 frames of the P2 target (120 fps) whatever the display is
//   fresh tick      a tick that has at least one canvas draw (a window rectangle put on a canvas by the page) since the previous tick: the canvas had NEW content for that screen frame
//   draw time       from the page receiving the Worker's frame message to its putImageData / layer paint being done (the JavaScript calls; the GPU's raster is not visible to a page)
//   transit         from the Worker posting the frame (its clock) to the page receiving it (two clocks of two threads: read it as about, not exact) — queue wait of the page's main thread included
//   per second      callbacks / OBSERVED seconds of the window: the visible seconds in which the meter was looking (a hidden tab is not counted as "0 fps")
// Decision 0202: the meter does NOT keep a requestAnimationFrame loop running. A page that asks for an animation frame at every frame makes Chrome hold pointer moves until the next frame, which added 10-20 ms to the input -> update
// delay of a page that otherwise draws nothing (decision 0202 section 4.2). So the rAF is asked for in short BURSTS: BURST_MS at the start, BURST_MS at the engine's first idle and every BURST_PERIOD_MS after it;
// outside a burst no rAF is requested. A burst's intervals are real intervals; the interval across two bursts is not counted (the chain is cut), and the rates divide by the observed (burst) seconds.
import { percentile } from '../ui/latency.ts';

export const RECENT_MS = 10_000;
export const LONGTASK_RECENT_MS = 30_000;
/** S-cbeacon1-bins: the exact intervals are kept for the last RECENT_MS only; "since the start" and "since the first idle" use a histogram with this bin width (0.05 ms = 0.6% of a 120 Hz frame); the interval histogram reaches the last edge of RAF_BIN_EDGES_MS so that its coarse bins are exact; at a bin edge the value belongs to the upper bin */
export const HIST_BIN_MS = 0.05;
export const RAF_RANGE_MS = 250;
export const DRAW_RANGE_MS = 20;
export const TRANSIT_RANGE_MS = 50;
/** the fixed bins of the beacon's interval histogram: counts of intervals <= each edge, and one more for everything above (so edges.length + 1 numbers). Edges are multiples of HIST_BIN_MS. */
export const RAF_BIN_EDGES_MS: readonly number[] = [6, 10.5, 14, 20, 30, 45, 70, 120, 250];
/** S-cbeacon1-slow */
export const SLOW_FACTOR = 1.5;
/** 1.5 frames of 120 fps */
export const SLOW_120_MS = 12.5;
/** the Event Timing API cannot report events shorter than this (the spec's minimum durationThreshold) */
export const EVENT_THRESHOLD_MS = 16;
const RING_CAP = 20_000;
/** S-cb2-13: the length of one burst of the rAF meter. 2 s = about 120 frames of a 60 Hz screen, 240 of a 120 Hz one (decision 0202) */
export const BURST_MS = 2000;
/** S-cb2-13: the time from one burst to the next */
export const BURST_PERIOD_MS = 60_000;
export type DisplayMode = 'burst' | 'continuous';

const round = (x: number, d: number): number => { const k = 10 ** d; return Math.round(x * k) / k; };
const r2 = (x: number | null): number | null => (x === null ? null : round(x, 2));

// ---- quantiles over exact values or over a histogram ----

export interface Q { n: number; mean: number | null; max: number | null; q(p: number): number | null; countAbove(x: number): number; countAtMost(x: number): number }

export class ArrayQ implements Q {
  readonly sorted: number[];
  constructor(values: readonly number[]) { this.sorted = values.slice().sort((a, b) => a - b); }
  get n(): number { return this.sorted.length; }
  get mean(): number | null { return this.n ? this.sorted.reduce((a, b) => a + b, 0) / this.n : null; }
  get max(): number | null { return this.n ? (this.sorted[this.n - 1] as number) : null; }
  q(p: number): number | null { return percentile(this.sorted, p); }
  countAbove(x: number): number { let c = 0; for (const v of this.sorted) if (v > x) c++; return c; }
  countAtMost(x: number): number { return this.n - this.countAbove(x); }
}

/** a histogram of non-negative values: bins of `binMs` up to `rangeMs`, one overflow count; the exact sum, maximum and count */
export class Hist implements Q {
  readonly counts: Uint32Array;
  n = 0;
  sum = 0;
  maxV = 0;
  over = 0;
  readonly binMs: number;
  readonly rangeMs: number;
  constructor(binMs: number, rangeMs: number) { this.binMs = binMs; this.rangeMs = rangeMs; this.counts = new Uint32Array(Math.ceil(rangeMs / binMs)); }
  add(v: number): void {
    if (!(v >= 0) || !Number.isFinite(v)) return;
    this.n++;
    this.sum += v;
    if (v > this.maxV) this.maxV = v;
    const i = Math.floor(v / this.binMs);
    if (i >= this.counts.length) this.over++;
    else this.counts[i] = (this.counts[i] as number) + 1;
  }
  get mean(): number | null { return this.n ? this.sum / this.n : null; }
  get max(): number | null { return this.n ? this.maxV : null; }
  /** nearest rank (as percentile()): the midpoint of the bin that holds the value of that rank; p = 100 is the exact maximum; a rank in the overflow answers the range's edge (the true value is at least that) */
  q(p: number): number | null {
    if (this.n === 0) return null;
    if (p >= 100) return this.maxV;
    const rank = Math.max(1, Math.ceil((p / 100) * this.n));
    let c = 0;
    for (let i = 0; i < this.counts.length; i++) { c += this.counts[i] as number; if (c >= rank) return (i + 0.5) * this.binMs; }
    return this.rangeMs;
  }
  /** values at or above x (bin granularity: x is taken to a bin edge) */
  countAbove(x: number): number {
    let c = this.over;
    for (let i = Math.max(0, Math.ceil(x / this.binMs - 1e-9)); i < this.counts.length; i++) c += this.counts[i] as number;
    return c;
  }
  countAtMost(x: number): number { return this.n - this.countAbove(x); }
}

export interface Q3 { n: number; p50: number | null; p95: number | null; max: number | null }
export function q3(q: Q): Q3 | null { return q.n === 0 ? null : { n: q.n, p50: r2(q.q(50)), p95: r2(q.q(95)), max: r2(q.max) }; }

// ---- the rAF side ----

export interface RafSummary {
  /** intervals counted */
  n: number;
  /** rAF callbacks per visible second (null: no visible time yet) */
  perSecond: number | null;
  medianMs: number | null; p95Ms: number | null; p99Ms: number | null; maxMs: number | null;
  /** 1000 / median interval: the rate the page was served */
  hz: number | null;
  /** 1000 / the 10th percentile interval: the rate of the fastest cluster */
  peakHz: number | null;
  /** intervals longer than SLOW_FACTOR x the median / longer than SLOW_120_MS */
  slow: number;
  over12p5: number;
  /** counts by RAF_BIN_EDGES_MS (<= each edge; the last: above the last edge) */
  bins: number[];
}

export function binCounts(q: Q): number[] {
  const out: number[] = [];
  let prev = 0;
  for (const e of RAF_BIN_EDGES_MS) { const c = q.countAtMost(e); out.push(c - prev); prev = c; }
  out.push(q.n - prev);
  return out;
}

/** `callbacks` and `visibleMs` give the per-second rate; the intervals give the rest */
export function summarizeRaf(q: Q, callbacks: number, visibleMs: number): RafSummary {
  const med = q.q(50), p10 = q.q(10);
  return {
    n: q.n,
    perSecond: visibleMs > 0 ? round((callbacks / visibleMs) * 1000, 2) : null,
    medianMs: r2(med), p95Ms: r2(q.q(95)), p99Ms: r2(q.q(99)), maxMs: r2(q.max),
    hz: med !== null && med > 0 ? round(1000 / med, 1) : null,
    peakHz: p10 !== null && p10 > 0 ? round(1000 / p10, 1) : null,
    slow: med === null ? 0 : q.countAbove(med * SLOW_FACTOR),
    over12p5: q.countAbove(SLOW_120_MS),
    bins: binCounts(q),
  };
}

// ---- the canvas side ----

export interface CanvasSummary {
  /** window rectangles put on a canvas by the page, in the window */
  draws: number;
  drawsPerSecond: number | null;
  /** rAF ticks in the window, and how many of them had at least one draw since the previous tick */
  ticks: number;
  freshTicks: number;
  freshRatio: number | null;
  /** canvas-updating frames per visible second */
  freshPerSecond: number | null;
  drawsPerFreshTick: number | null;
  drawMs: Q3 | null;
  transitMs: Q3 | null;
}

// ---- long tasks and slow input events ----

export interface LongTaskSummary { supported: boolean; total: number; maxMs: number | null; sumMs: number; sinceIdle: number | null; maxSinceIdleMs: number | null; recent30s: number }
export interface EventTimingSummary {
  supported: boolean;
  /** only events that took at least this long (input to the next paint, rounded to 8 ms by the browser) are reported by the API */
  thresholdMs: number;
  /** how many such slow events, of how many input events the page sent to the engine */
  slow: number;
  inputsSent: number;
  delayMs: Q3 | null;
  durationMs: Q3 | null;
  firstInput: { name: string; delayMs: number; durationMs: number } | null;
}

export interface VisibilitySummary { visibleMs: number; hiddenMs: number; changes: number; visibleNow: boolean }

export interface DisplaySnapshot {
  rafSupported: boolean;
  /** decision 0202: how the rAF was asked for ('burst': a burst of onMs at the start, at the first idle and every periodMs, `count` bursts so far, `recent10s` is the latest burst; 'continuous': a loop all the time, decision 0192's). A record without this field is continuous. */
  mode?: DisplayMode;
  burst?: { onMs: number; periodMs: number; count: number } | null;
  /** the meter has been running for this many ms of page time (0: not started) */
  runningMs: number;
  visibility: VisibilitySummary;
  raf: { recent10s: RafSummary; sinceIdle: RafSummary | null; total: RafSummary; binEdgesMs: number[] };
  /** null with display method A (the Worker draws; the page cannot see it) */
  canvas: { received: number; drawn: number; recent10s: CanvasSummary; sinceIdle: CanvasSummary | null; total: CanvasSummary; workerSent: number | null; workerOmitted: number | null } | null;
  longTasks: LongTaskSummary;
  events: EventTimingSummary;
}

interface Tick { t: number; dt: number | null; draws: number }
interface DrawRec { t: number; drawMs: number; transitMs: number | null }

/** everything accumulated over one stretch (the whole run, or since the first idle) */
class Acc {
  raf = new Hist(HIST_BIN_MS, RAF_RANGE_MS);
  drawMs = new Hist(HIST_BIN_MS, DRAW_RANGE_MS);
  transitMs = new Hist(HIST_BIN_MS, TRANSIT_RANGE_MS);
  callbacks = 0;
  ticks = 0;
  fresh = 0;
  drawsInTicks = 0;
  draws = 0;
  visibleBase = 0;
}

export class DisplayMeter {
  private startedAt: number | null = null;
  private visible = true;
  private visSince = 0;
  private visibleMs = 0;
  private hiddenMs = 0;
  private changes = 0;
  /** the visibility changes, for the visible time inside a window: [from, visible] */
  private readonly visLog: { t: number; visible: boolean }[] = [];
  /** decision 0202: false while no burst is on (burst mode between bursts); the meter looks at the screen only when it is visible AND observing */
  private observing = true;
  private obsMs = 0;
  private obsSince = 0;
  /** the changes of (visible && observing), for the observed time inside a window */
  private readonly obsLog: { t: number; on: boolean }[] = [];
  /** burst mode: the start of the latest burst and its end (null: still on / none) */
  private burstMode = false;
  private lastBurstStart: number | null = null;
  private burstsSeen = 0;
  private lastTick: number | null = null;
  private drawsSince = 0;
  private ticks: Tick[] = [];
  private tickHead = 0;
  private draws: DrawRec[] = [];
  private drawHead = 0;
  private receivedN = 0;
  private drawnN = 0;
  private readonly total = new Acc();
  private idle: Acc | null = null;

  /** the meter starts watching at `t` (page clock); `visible` is the page's state then */
  start(t: number, visible: boolean): void {
    if (this.startedAt !== null) return;
    this.startedAt = t;
    this.visible = visible;
    this.visSince = t;
    this.visLog.push({ t, visible });
    this.obsSince = t;
    this.obsLog.push({ t, on: visible && this.observing });
  }
  get started(): boolean { return this.startedAt !== null; }

  /** burst mode (decision 0202): the meter looks at the screen only between beginBurst() and endBurst(); call before start() */
  useBursts(): void { this.burstMode = true; this.observing = false; }

  private visibleUntil(now: number): number { return this.visibleMs + (this.visible ? Math.max(0, now - this.visSince) : 0); }
  private observedUntil(now: number): number { return this.obsMs + (this.visible && this.observing ? Math.max(0, now - this.obsSince) : 0); }
  /** (visible && observing) is about to change at `t`: close the observed stretch that was running */
  private obsChange(t: number, visible: boolean, observing: boolean): void {
    const was = this.visible && this.observing, will = visible && observing;
    if (was) this.obsMs += Math.max(0, t - this.obsSince);
    this.obsSince = t;
    if (was !== will) { this.obsLog.push({ t, on: will }); if (this.obsLog.length > 256) this.obsLog.splice(0, this.obsLog.length - 256); }
  }
  /** a burst starts at `t`: the tick chain is cut (the first tick of a burst starts no interval) */
  beginBurst(t: number): void {
    if (this.startedAt === null || this.observing) return;
    this.obsChange(t, this.visible, true);
    this.observing = true;
    this.lastBurstStart = t;
    this.burstsSeen++;
    this.lastTick = null;
    this.drawsSince = 0;
  }
  /** the burst ends at `t` */
  endBurst(t: number): void {
    if (this.startedAt === null || !this.observing) return;
    this.obsChange(t, this.visible, false);
    this.observing = false;
    this.lastTick = null;
    this.drawsSince = 0;
  }
  get bursts(): number { return this.burstsSeen; }

  /** the page became visible / hidden at `t`. The tick chain is cut: the first tick after it starts no interval. */
  setVisible(visible: boolean, t: number): void {
    if (this.startedAt === null) { this.start(t, visible); return; }
    if (visible === this.visible) return;
    const span = Math.max(0, t - this.visSince);
    this.obsChange(t, visible, this.observing);
    if (this.visible) this.visibleMs += span; else this.hiddenMs += span;
    this.visible = visible;
    this.visSince = t;
    this.changes++;
    this.lastTick = null;
    this.drawsSince = 0;
    this.visLog.push({ t, visible });
    if (this.visLog.length > 256) this.visLog.splice(0, this.visLog.length - 256);
  }

  /** the engine's first idle at `t`: "since the first idle" starts here */
  markIdle(t: number): void {
    if (this.idle) return;
    this.idle = new Acc();
    this.idle.visibleBase = this.observedUntil(t);
  }

  /** one requestAnimationFrame callback at page time `t` */
  tick(t: number): void {
    if (this.startedAt === null) this.start(t, true);
    if (!this.visible || !this.observing) return;
    const dt = this.lastTick === null ? null : t - this.lastTick;
    if (dt !== null && !(dt > 0)) return; // the same timestamp twice (or time going back): not a frame
    this.ticks.push({ t, dt, draws: this.drawsSince });
    this.evict(t);
    this.tickInto(this.total, dt);
    if (this.idle) this.tickInto(this.idle, dt);
    this.lastTick = t;
    this.drawsSince = 0;
  }

  private tickInto(a: Acc, dt: number | null): void {
    a.callbacks++;
    if (dt === null) return;
    a.raf.add(dt);
    a.ticks++;
    if (this.drawsSince > 0) a.fresh++;
    a.drawsInTicks += this.drawsSince;
  }

  private drawInto(a: Acc, dm: number, tm: number | null): void {
    a.draws++;
    a.drawMs.add(dm);
    if (tm !== null) a.transitMs.add(tm);
  }

  /** the page received a frame message from the Worker */
  received(): void { this.receivedN++; }

  /** the page drew a window rectangle: received at `recvAt`, drawn (the calls returned) at `drawnAt`, posted by the Worker at `postedAt` (its clock; undefined: unknown) */
  draw(recvAt: number, drawnAt: number, postedAt: number | undefined): void {
    this.drawnN++;
    this.drawsSince++;
    const dm = Math.max(0, drawnAt - recvAt);
    const tm = postedAt === undefined ? null : Math.max(0, recvAt - postedAt);
    this.draws.push({ t: drawnAt, drawMs: dm, transitMs: tm });
    this.evict(drawnAt);
    this.drawInto(this.total, dm, tm);
    if (this.idle) this.drawInto(this.idle, dm, tm);
  }

  private evict(now: number): void {
    const horizon = now - RECENT_MS - 1000;
    // burst mode: the ticks of the latest burst are the 'recent' window, however long ago it was
    const tickHorizon = this.burstMode && this.lastBurstStart !== null ? Math.min(horizon, this.lastBurstStart - 1) : horizon;
    while (this.tickHead < this.ticks.length && ((this.ticks[this.tickHead] as Tick).t < tickHorizon || this.ticks.length - this.tickHead > RING_CAP)) this.tickHead++;
    if (this.tickHead > 2048) { this.ticks = this.ticks.slice(this.tickHead); this.tickHead = 0; }
    while (this.drawHead < this.draws.length && ((this.draws[this.drawHead] as DrawRec).t < horizon || this.draws.length - this.drawHead > RING_CAP)) this.drawHead++;
    if (this.drawHead > 2048) { this.draws = this.draws.slice(this.drawHead); this.drawHead = 0; }
  }

  /** the observed time (ms: visible and, in burst mode, inside a burst) inside [from, to] */
  private observedWithin(from: number, to: number): number {
    let sum = 0;
    for (let i = 0; i < this.obsLog.length; i++) {
      const seg = this.obsLog[i] as { t: number; on: boolean };
      if (!seg.on) continue;
      const end = i + 1 < this.obsLog.length ? (this.obsLog[i + 1] as { t: number }).t : to;
      const a = Math.max(from, seg.t), b = Math.min(to, end);
      if (b > a) sum += b - a;
    }
    return sum;
  }

  visibility(now: number): VisibilitySummary {
    return { visibleMs: Math.round(this.visibleUntil(now)), hiddenMs: Math.round(this.hiddenMs + (this.visible ? 0 : Math.max(0, now - this.visSince))), changes: this.changes, visibleNow: this.visible };
  }

  private canvasOf(a: Acc): CanvasSummary {
    const tickSeconds = a.raf.sum / 1000;
    return {
      draws: a.draws,
      drawsPerSecond: tickSeconds > 0 ? round(a.drawsInTicks / tickSeconds, 2) : null,
      ticks: a.ticks, freshTicks: a.fresh,
      freshRatio: a.ticks > 0 ? round(a.fresh / a.ticks, 4) : null,
      freshPerSecond: tickSeconds > 0 ? round(a.fresh / tickSeconds, 2) : null,
      drawsPerFreshTick: a.fresh > 0 ? round(a.drawsInTicks / a.fresh, 2) : null,
      drawMs: q3(a.drawMs), transitMs: q3(a.transitMs),
    };
  }

  snapshot(now: number, canvasObservable: boolean, worker: { sent: number | null; omitted: number | null } = { sent: null, omitted: null }): Pick<DisplaySnapshot, 'runningMs' | 'visibility' | 'raf' | 'canvas'> {
    const started = this.startedAt ?? now;
    // the last RECENT_MS, exact; burst mode: the latest burst (what the meter saw last)
    const from = this.burstMode ? (this.lastBurstStart ?? now) - 1 : Math.max(started, now - RECENT_MS);
    const drawFrom = Math.max(started, now - RECENT_MS);
    const ticks = this.ticks.slice(this.tickHead).filter((x) => x.t > from && x.t <= now);
    const dts = ticks.flatMap((x) => (x.dt === null ? [] : [x.dt]));
    const rq = new ArrayQ(dts);
    const visRecent = this.observedWithin(from, now);
    const recentRaf = summarizeRaf(rq, ticks.length, visRecent);
    const dsum = dts.reduce((a, b) => a + b, 0) / 1000;
    const fresh = ticks.filter((x) => x.dt !== null && x.draws > 0).length;
    const drawsIn = ticks.reduce((a, x) => a + (x.dt === null ? 0 : x.draws), 0);
    const drawRecs = this.draws.slice(this.drawHead).filter((x) => x.t > drawFrom && x.t <= now);
    const recentCanvas: CanvasSummary = {
      draws: drawRecs.length,
      drawsPerSecond: dsum > 0 ? round(drawsIn / dsum, 2) : null,
      ticks: dts.length, freshTicks: fresh,
      freshRatio: dts.length > 0 ? round(fresh / dts.length, 4) : null,
      freshPerSecond: dsum > 0 ? round(fresh / dsum, 2) : null,
      drawsPerFreshTick: fresh > 0 ? round(drawsIn / fresh, 2) : null,
      drawMs: q3(new ArrayQ(drawRecs.map((x) => x.drawMs))),
      transitMs: q3(new ArrayQ(drawRecs.flatMap((x) => (x.transitMs === null ? [] : [x.transitMs])))),
    };
    const totalVis = this.observedUntil(now);
    const idleVis = this.idle ? totalVis - this.idle.visibleBase : 0;
    const raf = {
      recent10s: recentRaf,
      sinceIdle: this.idle ? summarizeRaf(this.idle.raf, this.idle.callbacks, idleVis) : null,
      total: summarizeRaf(this.total.raf, this.total.callbacks, totalVis),
      binEdgesMs: RAF_BIN_EDGES_MS.slice(),
    };
    return {
      runningMs: this.startedAt === null ? 0 : Math.round(now - this.startedAt),
      visibility: this.visibility(now),
      raf,
      canvas: canvasObservable ? { received: this.receivedN, drawn: this.drawnN, recent10s: recentCanvas, sinceIdle: this.idle ? this.canvasOf(this.idle) : null, total: this.canvasOf(this.total), workerSent: worker.sent, workerOmitted: worker.omitted } : null,
    };
  }
}

// ---- long tasks ----

export class LongTaskMeter {
  supported = false;
  private total = 0;
  private sum = 0;
  private max = 0;
  private sinceIdleN = 0;
  private sinceIdleMax = 0;
  private idleAt: number | null = null;
  private recent: { t: number; d: number }[] = [];
  /** a long-task entry: its start (page clock, abs ms) and duration */
  add(start: number, duration: number): void {
    this.total++;
    this.sum += duration;
    if (duration > this.max) this.max = duration;
    if (this.idleAt !== null && start >= this.idleAt) { this.sinceIdleN++; if (duration > this.sinceIdleMax) this.sinceIdleMax = duration; }
    this.recent.push({ t: start, d: duration });
    if (this.recent.length > 256) this.recent.shift();
  }
  markIdle(t: number): void { if (this.idleAt === null) this.idleAt = t; }
  snapshot(now: number): LongTaskSummary {
    return {
      supported: this.supported, total: this.total, maxMs: this.total ? round(this.max, 1) : null, sumMs: round(this.sum, 1),
      sinceIdle: this.idleAt === null ? null : this.sinceIdleN, maxSinceIdleMs: this.idleAt === null || this.sinceIdleN === 0 ? null : round(this.sinceIdleMax, 1),
      recent30s: this.recent.filter((x) => x.t > now - LONGTASK_RECENT_MS && x.t <= now).length,
    };
  }
}

// ---- Event Timing ----

export interface EventEntryLike { name: string; startTime: number; processingStart: number; duration: number }

export class EventTimingMeter {
  supported = false;
  private slowN = 0;
  private delays: number[] = [];
  private durations: number[] = [];
  private first: EventTimingSummary['firstInput'] = null;
  add(e: EventEntryLike): void {
    this.slowN++;
    this.delays.push(Math.max(0, e.processingStart - e.startTime));
    this.durations.push(e.duration);
    if (this.delays.length > 2000) { this.delays.shift(); this.durations.shift(); }
  }
  addFirstInput(e: EventEntryLike): void { this.first = { name: String(e.name).slice(0, 24), delayMs: round(Math.max(0, e.processingStart - e.startTime), 1), durationMs: round(e.duration, 1) }; }
  snapshot(inputsSent: number): EventTimingSummary {
    return { supported: this.supported, thresholdMs: EVENT_THRESHOLD_MS, slow: this.slowN, inputsSent, delayMs: q3(new ArrayQ(this.delays)), durationMs: q3(new ArrayQ(this.durations)), firstInput: this.first };
  }
}

// ---- the browser wiring ----

export interface PerfEntryLike { name: string; entryType: string; startTime: number; duration: number; processingStart?: number }
export interface DisplayHost {
  /** performance.timeOrigin + performance.now() */
  now(): number;
  /** performance.timeOrigin (a rAF timestamp + this = the page clock) */
  timeOrigin(): number;
  requestFrame(cb: (t: number) => void): void;
  /** decision 0202: runs `cb` once after `ms` (setTimeout); returns a handle for clearTimer */
  setTimer(cb: () => void, ms: number): unknown;
  clearTimer(handle: unknown): void;
  isVisible(): boolean;
  onVisibility(cb: (visible: boolean) => void): void;
  /** starts a PerformanceObserver for one entry type; false: the type is not supported here */
  observe(type: string, options: Record<string, unknown>, cb: (entries: PerfEntryLike[]) => void): boolean;
}

export interface DisplayMonitorOptions {
  /** 'burst' (default, decision 0202): requestAnimationFrame only in bursts. 'continuous': a rAF loop all the time (decision 0192's; kept for comparison and for the tests) */
  mode?: DisplayMode;
  burstMs?: number;
  periodMs?: number;
}

export class DisplayMonitor {
  readonly meter = new DisplayMeter();
  readonly longTasks = new LongTaskMeter();
  readonly events = new EventTimingMeter();
  readonly mode: DisplayMode;
  readonly burstMs: number;
  readonly periodMs: number;
  private running = false;
  private frameLoop = false;
  private idleMarked = false;
  private idleBurstDone = false;
  private bursting = false;
  private burstTimer: unknown = null;
  private periodTimer: unknown = null;
  private readonly host: DisplayHost;
  constructor(host: DisplayHost, opts: DisplayMonitorOptions = {}) {
    this.host = host;
    this.mode = opts.mode ?? 'burst';
    this.burstMs = opts.burstMs ?? BURST_MS;
    this.periodMs = opts.periodMs ?? BURST_PERIOD_MS;
  }

  get isRunning(): boolean { return this.running; }

  /** starts the rAF meter (a burst now, then the schedule below; or the continuous loop) and the observers; idempotent */
  start(): void {
    if (this.running) return;
    this.running = true;
    const h = this.host;
    if (this.mode === 'burst') this.meter.useBursts();
    this.meter.start(h.now(), h.isVisible());
    h.onVisibility((v) => this.meter.setVisible(v, h.now()));
    if (this.mode === 'continuous') {
      this.frameLoop = true;
      const loop = (t: number): void => { this.meter.tick(t + h.timeOrigin()); h.requestFrame(loop); };
      h.requestFrame(loop);
    } else {
      this.beginBurst();
      this.schedulePeriod();
    }
    const origin = (): number => h.timeOrigin();
    this.longTasks.supported = h.observe('longtask', { buffered: true }, (list) => { for (const e of list) this.longTasks.add(e.startTime + origin(), e.duration); });
    this.events.supported = h.observe('event', { buffered: true, durationThreshold: EVENT_THRESHOLD_MS }, (list) => { for (const e of list) this.events.add({ name: e.name, startTime: e.startTime, processingStart: e.processingStart ?? e.startTime, duration: e.duration }); });
    h.observe('first-input', { buffered: true }, (list) => { for (const e of list) this.events.addFirstInput({ name: e.name, startTime: e.startTime, processingStart: e.processingStart ?? e.startTime, duration: e.duration }); });
    if (this.idleMarked) this.markIdle();
  }

  /** burst mode: one burst of burstMs (a burst already on is made to last burstMs from now). Outside a burst no requestAnimationFrame is asked for. */
  private beginBurst(): void {
    if (this.mode !== 'burst' || !this.running) return;
    const h = this.host;
    if (this.burstTimer !== null) h.clearTimer(this.burstTimer);
    this.burstTimer = h.setTimer(() => this.endBurst(), this.burstMs);
    if (this.bursting) return;
    this.bursting = true;
    this.frameLoop = true;
    this.meter.beginBurst(h.now());
    const loop = (t: number): void => {
      if (!this.bursting) return;   // the burst ended while this callback was pending: it neither counts nor asks for another
      this.meter.tick(t + h.timeOrigin());
      h.requestFrame(loop);
    };
    h.requestFrame(loop);
  }
  private endBurst(): void {
    this.burstTimer = null;
    if (!this.bursting) return;
    this.bursting = false;
    this.frameLoop = false;
    this.meter.endBurst(this.host.now());
  }
  /** the next burst after periodMs, and so on */
  private schedulePeriod(): void {
    if (this.periodTimer !== null) this.host.clearTimer(this.periodTimer);
    this.periodTimer = this.host.setTimer(() => { this.beginBurst(); this.schedulePeriod(); }, this.periodMs);
  }

  /** the engine's first idle: the "since the first idle" windows start now (if the monitor starts later, they start then) */
  markIdle(): void {
    this.idleMarked = true;
    if (!this.running) return;
    const t = this.host.now();
    this.meter.markIdle(t);
    this.longTasks.markIdle(t);
    // burst mode: the burst that follows the first idle, and the schedule counts from here
    if (this.mode === 'burst' && !this.idleBurstDone) { this.idleBurstDone = true; this.beginBurst(); this.schedulePeriod(); }
  }
  /** true while a requestAnimationFrame loop is going (burst mode: during a burst only) */
  get rafLoopRunning(): boolean { return this.frameLoop; }

  snapshot(canvasObservable: boolean, inputsSent: number, worker?: { sent: number | null; omitted: number | null }): DisplaySnapshot {
    const now = this.host.now();
    const m = this.meter.snapshot(now, canvasObservable, worker);
    return { rafSupported: this.running, mode: this.mode, burst: this.mode === 'burst' ? { onMs: this.burstMs, periodMs: this.periodMs, count: this.meter.bursts } : null, runningMs: m.runningMs, visibility: m.visibility, raf: m.raf, canvas: m.canvas, longTasks: this.longTasks.snapshot(now), events: this.events.snapshot(inputsSent) };
  }
}

// ---- reading the numbers ----

export const DISPLAY_TARGET_HZ = 120;
/** S-cbeacon1-hz-met: 95% of the 120 Hz target; a 120 Hz display reports 119.9..120.1 */
export const DISPLAY_MET_HZ = 0.95 * DISPLAY_TARGET_HZ;

/** a name for a measured rate: "60 Hz class" and so on (S-cbeacon1-class: the bands halfway between the common rates); null below 50 Hz, where the page is being throttled rather than served by a display of that rate */
export function hzClass(hz: number | null): number | null {
  if (hz === null || !(hz >= 50)) return null;
  const rates = [60, 75, 90, 100, 120, 144, 165, 240, 360];
  let best = rates[0] as number, bd = Infinity;
  for (const r of rates) { const d = Math.abs(Math.log(hz / r)); if (d < bd) { bd = d; best = r; } }
  return bd < 0.06 ? best : null;
}

export type HzFitState = 'match' | 'engine-slower' | 'display-slower' | 'page-behind' | 'unknown';
export interface HzInfo {
  /** the fields of beacon version 3 (the server still accepts the other values of records stored earlier): the page measures the rate (auto) and passes no parameter */
  param: null;
  request: 'auto';
  /** what the page put into FLE_DWM_HZ / FLE_DISPLAYSETTINGS_FREQUENCY (null: nothing — the engine runs at its own 60) */
  engineEnvHz: number | null;
  /** the rate the engine is paced by: engineEnvHz, else the engine's default 60 */
  engineHz: number;
  /** the page's own measurement of the display rate (null: not enough frames) */
  autoMeasuredHz: number | null;
  /** the measured rAF rate and the peak rate used in the comparison (since the first idle if there is one, else all) and which window that was */
  rafHz: number | null;
  peakHz: number | null;
  basis: 'since-idle' | 'total' | null;
  /** engine Hz / rAF Hz */
  ratio: number | null;
  fit: HzFitState;
  note: string;
}

export interface HzRequestFacts { engineEnvHz: number | null; engineHz: number; autoMeasuredHz: number | null }

/**
 * Does the engine's rate fit the screen? ratio = engine Hz / rAF Hz. Within 10%: match. The engine slower than the screen: the screen is not used (a 120 Hz screen with no ?hz=120). The engine faster: the screen shows at most its own rate.
 * If the page's rAF is far below the fastest cluster of its own intervals (peak Hz > 1.3 x rAF Hz) the page is not keeping up: that is not a statement about the display (state 'page-behind'). S-cbeacon1-fit: the 10% and 1.3.
 */
export function hzFit(f: HzRequestFacts, raf: DisplaySnapshot['raf']): HzInfo {
  const s = raf.sinceIdle && raf.sinceIdle.n >= 30 ? raf.sinceIdle : null;
  const basisSummary = s ?? (raf.total.n >= 30 ? raf.total : null);
  const rafHz = basisSummary?.hz ?? null, peakHz = basisSummary?.peakHz ?? null;
  const base = { param: null, request: 'auto' as const, engineEnvHz: f.engineEnvHz, engineHz: f.engineHz, autoMeasuredHz: f.autoMeasuredHz, rafHz, peakHz, basis: basisSummary ? (s ? 'since-idle' as const : 'total' as const) : null };
  if (rafHz === null) return { ...base, ratio: null, fit: 'unknown', note: 'rAF の間隔がまだ足りません' };
  const ratio = round(f.engineHz / rafHz, 3);
  if (peakHz !== null && peakHz > 1.3 * rafHz) return { ...base, ratio, fit: 'page-behind', note: `画面は約 ${Math.round(peakHz)} Hz ですが、ページの rAF は ${rafHz.toFixed(0)} 回/s しか来ていません(ページが間に合っていません)` };
  if (ratio < 0.9) return { ...base, ratio, fit: 'engine-slower', note: `この画面は ${rafHz.toFixed(0)} Hz 相当ですが、エンジンは ${f.engineHz} Hz で描画しています` };
  if (ratio > 1.1) return { ...base, ratio, fit: 'display-slower', note: `エンジンは ${f.engineHz} Hz で描画しますが、この画面の rAF は ${rafHz.toFixed(0)} 回/s なので、見えるのは最大 ${rafHz.toFixed(0)} fps です` };
  return { ...base, ratio, fit: 'match', note: `エンジン ${f.engineHz} Hz と画面の rAF ${rafHz.toFixed(0)} Hz は合っています` };
}
