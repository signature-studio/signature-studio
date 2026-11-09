// Decision 0192: the screen-side meters (src/perf/display.ts). Everything is driven with made-up timestamps / a fake host: 60 / 120 / 144 Hz, jitter, a hidden stretch, one long stall, canvas draws between frames.
import { describe, expect, it } from 'vitest';
import { ArrayQ, DisplayMeter, RAF_RANGE_MS, DisplayMonitor, EventTimingMeter, Hist, LongTaskMeter, RAF_BIN_EDGES_MS, binCounts, hzClass, hzFit, summarizeRaf, type DisplayHost, type PerfEntryLike } from '../../src/perf/display.ts';
import { beaconProblems } from '../../shared/perf.ts';

const T0 = 1_700_000_000_000;

/** a deterministic pseudo-random in [-1, 1) (a small LCG; the tests need jitter, not randomness) */
function rng(seed: number): () => number {
  let x = seed >>> 0;
  return () => { x = (Math.imul(x, 1664525) + 1013904223) >>> 0; return x / 2 ** 31 - 1; };
}

/** ticks at `hz` for `seconds` starting at `from`; returns the time of the last tick */
function feed(m: DisplayMeter, hz: number, seconds: number, from: number, jitterMs = 0, seed = 1): number {
  const r = rng(seed), period = 1000 / hz, n = Math.round(seconds * hz);
  let t = from;
  for (let i = 0; i < n; i++) { t = from + i * period + (jitterMs ? r() * jitterMs : 0); m.tick(t); }
  return t;
}

describe('estimating the display rate from requestAnimationFrame intervals', () => {
  it.each([[60, 16.67], [120, 8.33], [144, 6.94], [75, 13.33], [240, 4.17]])('a steady %d Hz is estimated as %d Hz (to 0.5%) over the last 10 s and since the start', (hz) => {
    const m = new DisplayMeter();
    m.start(T0, true);
    const last = feed(m, hz, 20, T0);
    const s = m.snapshot(last, true).raf;
    for (const w of [s.recent10s, s.total]) {
      expect(Math.abs((w.hz as number) - hz) / hz).toBeLessThan(0.005);
      expect(Math.abs((w.peakHz as number) - hz) / hz).toBeLessThan(0.005);
      expect(w.slow).toBe(0);
      expect(w.perSecond).toBeGreaterThan(hz * 0.99);
      expect(w.perSecond).toBeLessThan(hz * 1.01);
    }
    expect(s.recent10s.n).toBeGreaterThanOrEqual(10 * hz - 2);
    expect(s.recent10s.n).toBeLessThanOrEqual(10 * hz + 1);
  });
  it('with +-0.3 ms of jitter the estimate is still within 2% and no frame is called slow', () => {
    for (const hz of [60, 120, 144]) {
      const m = new DisplayMeter();
      m.start(T0, true);
      const last = feed(m, hz, 20, T0, 0.6, hz);
      const w = m.snapshot(last, true).raf.recent10s;
      expect(Math.abs((w.hz as number) - hz) / hz).toBeLessThan(0.02);
      expect(w.slow).toBe(0);
    }
  });
  it('a 120 Hz screen served only partly: 15% of the intervals are one frame (8.3 ms), the rest two frames (16.7 ms) -> the median is the page\'s 60, the peak is the screen\'s 120, and the page is "behind"', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = T0;
    for (let i = 0; i < 2000; i++) { t += i % 20 < 3 ? 1000 / 120 : 1000 / 60; m.tick(t); }
    const w = m.snapshot(t, true).raf.total;
    expect(Math.abs((w.hz as number) - 60)).toBeLessThan(0.5);
    expect(Math.abs((w.peakHz as number) - 120)).toBeLessThan(1);
    expect(w.slow).toBe(0);
    const fit = hzFit({ engineEnvHz: 120, engineHz: 120, autoMeasuredHz: null }, m.snapshot(t, true).raf);
    expect(fit.fit).toBe('page-behind');
  });
  it('a page on time 70% of the time (8.3 ms) and a frame late 30% (16.7 ms) is a 120 Hz page: the median follows the majority, not the slowest tail', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = T0;
    for (let i = 0; i < 2000; i++) { t += i % 10 < 7 ? 1000 / 120 : 1000 / 60; m.tick(t); }
    const w = m.snapshot(t, true).raf.total;
    expect(Math.abs((w.hz as number) - 120)).toBeLessThan(1);
    expect(w.slow).toBe(600);
  });
  it('one long stop (500 ms between two frames) is one slow interval: it moves the maximum and the count, not the median or the rate class', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = feed(m, 120, 4, T0);
    t += 500;
    m.tick(t);
    t = feed(m, 120, 4, t + 1000 / 120);
    const w = m.snapshot(t, true).raf.total;
    expect(w.maxMs).toBeGreaterThan(499);
    expect(w.maxMs).toBeLessThan(502);
    expect(w.slow).toBe(1);
    expect(w.over12p5).toBe(1);
    expect(Math.abs((w.hz as number) - 120)).toBeLessThan(0.7);
    expect(w.n).toBe(8 * 120);
  });
  it('a hidden stretch is not a slow frame: rAF stops in a hidden tab, the interval over it is dropped, and the visible time does not include it', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = feed(m, 60, 5, T0);
    m.setVisible(false, t + 5);
    const back = t + 30_000;
    m.setVisible(true, back);
    t = feed(m, 60, 5, back + 3);
    const s = m.snapshot(t, true);
    expect(s.raf.total.maxMs).toBeLessThan(17);
    expect(s.raf.total.slow).toBe(0);
    expect(s.raf.total.n).toBe(5 * 60 - 1 + 5 * 60 - 1);
    expect(s.visibility.changes).toBe(2);
    expect(s.visibility.hiddenMs).toBeGreaterThan(29_990);
    expect(s.visibility.hiddenMs).toBeLessThan(30_010);
    expect(s.visibility.visibleMs).toBeGreaterThan(9_900);
    expect(s.visibility.visibleMs).toBeLessThan(10_100);
    // callbacks per VISIBLE second: about 60, not 600 / 40
    expect(s.raf.total.perSecond).toBeGreaterThan(59);
    expect(s.raf.total.perSecond).toBeLessThan(61);
  });
  it('the same silence WITHOUT a visibility change (a frozen page) is a 30 s interval: it is counted, not hidden', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = feed(m, 60, 5, T0);
    t = feed(m, 60, 5, t + 30_000);
    const w = m.snapshot(t, true).raf.total;
    expect(w.maxMs).toBeGreaterThan(29_000);
    expect(w.slow).toBe(1);
    expect(w.perSecond as number).toBeLessThan(16);
  });
  it('ticks that arrive while the page is hidden are ignored (some browsers fire rAF about once a second in a hidden tab)', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = feed(m, 60, 2, T0);
    m.setVisible(false, t);
    for (let i = 1; i <= 5; i++) m.tick(t + i * 1000);
    m.setVisible(true, t + 5200);
    t = feed(m, 60, 2, t + 5300);
    const w = m.snapshot(t, true).raf.total;
    expect(w.maxMs).toBeLessThan(17);
    expect(w.n).toBe(2 * 60 - 1 + 2 * 60 - 1);
  });
  it('the last 10 s follow a change of rate (60 Hz, then 120 Hz) while the whole run shows the blend', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = feed(m, 60, 15, T0);
    t = feed(m, 120, 12, t + 1000 / 120);
    const s = m.snapshot(t, true).raf;
    expect(Math.abs((s.recent10s.hz as number) - 120)).toBeLessThan(1);
    expect(s.total.perSecond as number).toBeGreaterThan(80);
    expect(s.total.perSecond as number).toBeLessThan(100);
  });
  it('"since the first idle" starts at markIdle: what came before is not in it', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    let t = feed(m, 30, 3, T0); // a slow start-up (30 Hz while the page decompresses)
    m.markIdle(t);
    t = feed(m, 120, 5, t + 8.33);
    const s = m.snapshot(t, true).raf;
    expect(Math.abs((s.sinceIdle?.hz as number) - 120)).toBeLessThan(1);
    expect(s.sinceIdle?.n).toBe(5 * 120 - 1 + 1);
    expect(s.total.n).toBeGreaterThan(s.sinceIdle?.n as number);
    expect(m.snapshot(t, true).raf.total.maxMs).toBeLessThan(35);
  });
  it('before any idle there is no "since idle" window; before any tick there is no rate (null, not 0)', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    const s = m.snapshot(T0 + 1000, true);
    expect(s.raf.sinceIdle).toBeNull();
    expect(s.raf.recent10s.hz).toBeNull();
    expect(s.raf.recent10s.medianMs).toBeNull();
    expect(s.raf.recent10s.n).toBe(0);
    // a visible page that got no frame at all in a second is 0 frames per second, which is a fact
    expect(s.raf.recent10s.perSecond).toBe(0);
  });
  it('the exact (last 10 s) and the histogram (whole run) statistics agree to the histogram bin on the same intervals', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    const last = feed(m, 120, 9, T0, 1.2, 7);
    const s = m.snapshot(last, true).raf;
    expect(Math.abs((s.recent10s.medianMs as number) - (s.total.medianMs as number))).toBeLessThan(0.06);
    expect(Math.abs((s.recent10s.p95Ms as number) - (s.total.p95Ms as number))).toBeLessThan(0.06);
    expect(s.recent10s.bins).toEqual(s.total.bins);
  });
  it('memory is bounded: ten minutes at 240 Hz keep only about the last 11 s of frames', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    const last = feed(m, 240, 600, T0);
    const priv = m as unknown as { ticks: unknown[]; tickHead: number };
    expect(priv.ticks.length - priv.tickHead).toBeLessThan(240 * 11 + 50);
    expect(m.snapshot(last, true).raf.total.n).toBe(600 * 240 - 1);
  });
  it('ignores the same timestamp twice and time going back', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    m.tick(T0 + 10); m.tick(T0 + 10); m.tick(T0 + 5); m.tick(T0 + 26.67);
    expect(m.snapshot(T0 + 30, true).raf.total.n).toBe(1);
  });
});

describe('the bins and the quantiles', () => {
  it('RAF_BIN_EDGES_MS has 9 edges (10 counts) and every edge is a multiple of the histogram bin', () => {
    expect(RAF_BIN_EDGES_MS).toHaveLength(9);
    for (const e of RAF_BIN_EDGES_MS) expect(Math.abs(Math.round(e / 0.05) * 0.05 - e)).toBeLessThan(1e-9);
  });
  it('binCounts is the same for exact values and for the histogram', () => {
    const v = [3, 5.9, 6.05, 6.1, 8.3, 10.45, 10.6, 16.7, 16.7, 25, 33.4, 50.2, 69.9, 119, 300];
    const h = new Hist(0.05, RAF_RANGE_MS);
    for (const x of v) h.add(x);
    expect(binCounts(new ArrayQ(v))).toEqual([2, 4, 1, 2, 1, 1, 2, 1, 0, 1]);
    expect(binCounts(h)).toEqual(binCounts(new ArrayQ(v)));
    expect(binCounts(new ArrayQ(v)).reduce((a, b) => a + b, 0)).toBe(v.length);
  });
  it('Hist: nearest-rank quantiles to the bin, the exact maximum, overflow above the range', () => {
    const h = new Hist(0.05, 50);
    for (let i = 1; i <= 100; i++) h.add(i / 10); // 0.1 .. 10.0
    expect(Math.abs((h.q(50) as number) - 5.0)).toBeLessThan(0.06);
    expect(Math.abs((h.q(95) as number) - 9.5)).toBeLessThan(0.06);
    expect(h.q(100)).toBe(10);
    h.add(80);
    expect(h.max).toBe(80);
    expect(h.countAbove(50)).toBe(1);
    expect(h.q(100)).toBe(80);
    expect(new Hist(0.05, 50).q(50)).toBeNull();
  });
  it('Hist ignores NaN, negative and infinite values', () => {
    const h = new Hist(0.05, 50);
    h.add(NaN); h.add(-1); h.add(Infinity);
    expect(h.n).toBe(0);
  });
  it('summarizeRaf: slow = longer than 1.5 x the median; over12p5 = longer than 12.5 ms whatever the median is', () => {
    const q = new ArrayQ([...Array(50).fill(16.67), 25.1, 26, 12.6]);
    const s = summarizeRaf(q, 53, 1000);
    expect(s.medianMs).toBe(16.67);
    expect(s.slow).toBe(2);
    expect(s.over12p5).toBe(53);
    const q2 = new ArrayQ([...Array(50).fill(8.33), 12.4, 12.6, 20]);
    const s2 = summarizeRaf(q2, 53, 1000);
    expect(s2.slow).toBe(2);
    expect(s2.over12p5).toBe(2);
  });
});

describe('the canvas side: was the canvas really updated in each screen frame?', () => {
  /** frames at `screenHz`, `drawHz` draws a second spread evenly in between */
  function run(screenHz: number, drawHz: number, seconds: number, opts: { drawMs?: number; transit?: number } = {}): { m: DisplayMeter; last: number } {
    const m = new DisplayMeter();
    m.start(T0, true);
    const end = seconds * 1000, tickP = 1000 / screenHz, drawP = 1000 / drawHz;
    let nextDraw = drawP / 2, last = T0;
    for (let k = 1; k * tickP <= end; k++) {
      const t = T0 + k * tickP;
      while (nextDraw <= k * tickP - 1e-6) {
        const at = T0 + nextDraw;
        const recv = at - (opts.drawMs ?? 0.4);
        m.received();
        m.draw(recv, at, recv - (opts.transit ?? 1.2));
        nextDraw += drawP;
      }
      m.tick(t);
      last = t;
    }
    return { m, last };
  }
  it('120 draws a second on a 120 Hz screen: every screen frame has new content, one draw each', () => {
    const { m, last } = run(120, 120, 12);
    const c = m.snapshot(last, true).canvas!.recent10s;
    expect(c.freshRatio).toBeGreaterThan(0.98);
    expect(c.drawsPerFreshTick).toBeGreaterThan(0.99);
    expect(c.drawsPerFreshTick).toBeLessThan(1.02);
    expect(c.freshPerSecond).toBeGreaterThan(117);
  });
  it('125 draws a second on a 60 Hz screen (the engine at 120 Hz, a 60 Hz display): 60 fresh frames a second, about two draws in each — the screen cannot show more', () => {
    const { m, last } = run(60, 125, 12);
    const c = m.snapshot(last, true).canvas!.recent10s;
    expect(c.freshRatio).toBe(1);
    expect(c.freshPerSecond).toBeGreaterThan(59);
    expect(c.freshPerSecond).toBeLessThan(61);
    expect(c.drawsPerFreshTick).toBeGreaterThan(2);
    expect(c.drawsPerFreshTick).toBeLessThan(2.2);
    expect(c.drawsPerSecond).toBeGreaterThan(123);
  });
  it('30 draws a second on a 120 Hz screen: a quarter of the screen frames are fresh', () => {
    const { m, last } = run(120, 30, 12);
    const c = m.snapshot(last, true).canvas!.recent10s;
    expect(c.freshRatio).toBeGreaterThan(0.24);
    expect(c.freshRatio).toBeLessThan(0.26);
  });
  it('no draws at all: fresh ratio 0 and no draw time (null, not zero)', () => {
    const { m, last } = run(60, 0.0001, 5);
    const c = m.snapshot(last, true).canvas!.recent10s;
    expect(c.freshTicks).toBe(0);
    expect(c.freshRatio).toBe(0);
    expect(c.drawMs).toBeNull();
    expect(c.drawsPerFreshTick).toBeNull();
  });
  it('the time from receiving a frame to having drawn it, and the Worker-to-page transit, are measured separately', () => {
    const { m, last } = run(60, 60, 5, { drawMs: 2.5, transit: 3.5 });
    const c = m.snapshot(last, true).canvas!;
    expect(c.recent10s.drawMs?.p50).toBeGreaterThan(2.4);
    expect(c.recent10s.drawMs?.p50).toBeLessThan(2.6);
    expect(c.recent10s.transitMs?.p95).toBeGreaterThan(3.4);
    expect(c.recent10s.transitMs?.p95).toBeLessThan(3.6);
    expect(c.total.drawMs?.p95).toBeGreaterThan(2.4);
    expect(c.received).toBe(c.drawn);
  });
  it('a frame with no known post time has no transit; a negative difference between the two clocks is 0, not a negative time', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    m.received(); m.draw(T0 + 1, T0 + 1.5, undefined);
    m.received(); m.draw(T0 + 2, T0 + 2.5, T0 + 9);
    m.tick(T0 + 16); m.tick(T0 + 32);
    const c = m.snapshot(T0 + 40, true).canvas!.recent10s;
    expect(c.transitMs).toEqual({ n: 1, p50: 0, p95: 0, max: 0 });
    expect(c.drawMs?.n).toBe(2);
  });
  it('received counts every frame message, drawn only the ones that were drawn; the page can tell the Worker\'s own numbers (dedupe) next to them', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    m.received(); m.received(); m.received();
    m.draw(T0, T0 + 1, undefined);
    const c = m.snapshot(T0 + 10, true, { sent: 100, omitted: 900 }).canvas!;
    expect(c.received).toBe(3);
    expect(c.drawn).toBe(1);
    expect(c.workerSent).toBe(100);
    expect(c.workerOmitted).toBe(900);
  });
  it('display method A (the Worker draws): no canvas block at all', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    expect(m.snapshot(T0 + 10, false).canvas).toBeNull();
  });
  it('draws made while the page is hidden do not make the first frame after it look fresh', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    m.tick(T0 + 16.67);
    m.setVisible(false, T0 + 20);
    for (let i = 0; i < 50; i++) { m.received(); m.draw(T0 + 30 + i, T0 + 30.5 + i, undefined); }
    m.setVisible(true, T0 + 5000);
    m.tick(T0 + 5010); m.tick(T0 + 5026.67);
    const c = m.snapshot(T0 + 5030, true).canvas!;
    expect(c.total.freshTicks).toBe(0);
    expect(c.drawn).toBe(50);
  });
});

describe('long tasks and slow input events', () => {
  it('counts long tasks, the largest, those since the first idle and those in the last 30 s', () => {
    const l = new LongTaskMeter();
    l.supported = true;
    l.add(T0 + 1000, 120);
    l.add(T0 + 2000, 60);
    l.markIdle(T0 + 3000);
    l.add(T0 + 4000, 75);
    l.add(T0 + 40_000, 300);
    const s = l.snapshot(T0 + 50_000);
    expect(s).toMatchObject({ supported: true, total: 4, maxMs: 300, sinceIdle: 2, maxSinceIdleMs: 300, recent30s: 1 });
    expect(new LongTaskMeter().snapshot(T0)).toMatchObject({ supported: false, total: 0, maxMs: null, sinceIdle: null });
  });
  it('a long task that started BEFORE the first idle but was delivered after it (the observer is asynchronous) is not "since the first idle"', () => {
    const l = new LongTaskMeter();
    l.supported = true;
    l.markIdle(T0 + 3000);
    l.add(T0 + 2900, 400);
    l.add(T0 + 3100, 70);
    const s = l.snapshot(T0 + 4000);
    expect(s).toMatchObject({ total: 2, maxMs: 400, sinceIdle: 1, maxSinceIdleMs: 70 });
  });
  it('slow events: input delay and duration quantiles, and the first input', () => {
    const e = new EventTimingMeter();
    e.supported = true;
    for (let i = 0; i < 20; i++) e.add({ name: 'pointermove', startTime: 100 * i, processingStart: 100 * i + (i + 1), duration: 16 + 8 * (i % 3) });
    e.addFirstInput({ name: 'pointerdown', startTime: 10, processingStart: 14.4, duration: 24 });
    const s = e.snapshot(500);
    expect(s.slow).toBe(20);
    expect(s.inputsSent).toBe(500);
    expect(s.delayMs).toEqual({ n: 20, p50: 10, p95: 19, max: 20 });
    expect(s.durationMs?.max).toBe(32);
    expect(s.firstInput).toEqual({ name: 'pointerdown', delayMs: 4.4, durationMs: 24 });
    expect(new EventTimingMeter().snapshot(0)).toMatchObject({ supported: false, slow: 0, delayMs: null, firstInput: null });
  });
});

describe('the monitor wired to a fake browser', () => {
  class Fake implements DisplayHost {
    t = 0; // ms since the origin
    cbs: ((t: number) => void)[] = [];
    vis = true;
    visCbs: ((v: boolean) => void)[] = [];
    observers = new Map<string, { opts: Record<string, unknown>; cb: (e: PerfEntryLike[]) => void }>();
    supported = new Set(['longtask', 'event', 'first-input']);
    now(): number { return T0 + this.t; }
    timeOrigin(): number { return T0; }
    requestFrame(cb: (t: number) => void): void { this.cbs.push(cb); }
    timers: { id: number; at: number; cb: () => void }[] = [];
    nextTimer = 1;
    setTimer(cb: () => void, ms: number): unknown { const id = this.nextTimer++; this.timers.push({ id, at: this.t + ms, cb }); return id; }
    clearTimer(handle: unknown): void { this.timers = this.timers.filter((x) => x.id !== handle); }
    isVisible(): boolean { return this.vis; }
    onVisibility(cb: (v: boolean) => void): void { this.visCbs.push(cb); }
    observe(type: string, opts: Record<string, unknown>, cb: (e: PerfEntryLike[]) => void): boolean { if (!this.supported.has(type)) return false; this.observers.set(type, { opts, cb }); return true; }
    /** one vsync: runs the callbacks that were asked for (only while the page is visible) */
    vsync(periodMs: number): void {
      this.t += periodMs;
      // the timers that are due run first, in order (a timer is a task of its own, before the frame)
      for (;;) { const due = this.timers.filter((x) => x.at <= this.t).sort((a, b) => a.at - b.at)[0]; if (!due) break; this.timers = this.timers.filter((x) => x !== due); due.cb(); }
      if (!this.vis) return;
      const run = this.cbs.splice(0);
      for (const cb of run) cb(this.t);
    }
    setVis(v: boolean): void { this.vis = v; for (const c of this.visCbs) c(v); }
  }
  it('start() asks for frames and observers once; an observer type the browser lacks is "not supported", not an error', () => {
    const h = new Fake();
    h.supported.delete('longtask');
    const m = new DisplayMonitor(h, { mode: 'continuous' });
    m.start(); m.start();
    expect(h.cbs).toHaveLength(1);
    expect(h.visCbs).toHaveLength(1);
    expect(m.longTasks.supported).toBe(false);
    expect(m.events.supported).toBe(true);
    expect(h.observers.get('event')?.opts).toEqual({ buffered: true, durationThreshold: 16 });
    expect(m.snapshot(true, 0).longTasks.supported).toBe(false);
  });
  it('reads a 120 Hz run, a hidden stretch and entries from the observers', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h, { mode: 'continuous' });
    m.start();
    for (let i = 0; i < 600; i++) h.vsync(1000 / 120);
    h.setVis(false);
    for (let i = 0; i < 600; i++) h.vsync(1000 / 120); // no callbacks while hidden
    h.setVis(true);
    h.vsync(1000 / 120);
    for (let i = 0; i < 600; i++) h.vsync(1000 / 120);
    h.observers.get('longtask')?.cb([{ name: 'self', entryType: 'longtask', startTime: 2000, duration: 88 }]);
    h.observers.get('event')?.cb([{ name: 'pointerdown', entryType: 'event', startTime: 3000, processingStart: 3006, duration: 32 }]);
    const s = m.snapshot(true, 12);
    expect(Math.abs((s.raf.total.hz as number) - 120)).toBeLessThan(0.7);
    expect(s.raf.total.slow).toBe(0);
    expect(s.visibility).toMatchObject({ changes: 2, visibleNow: true });
    expect(s.visibility.hiddenMs).toBeGreaterThan(4900);
    expect(s.longTasks).toMatchObject({ supported: true, total: 1, maxMs: 88 });
    expect(s.events).toMatchObject({ supported: true, slow: 1, inputsSent: 12 });
    expect(s.events.delayMs?.p50).toBe(6);
    expect(beaconProblems({ v: 2, sid: 'a1b2c3d4e5f6', display: s, hz: null, client: null })).toEqual([]);
    expect(s.mode).toBe('continuous');
    expect(s.burst).toBeNull();
  });
  it('markIdle before start is remembered: the since-idle window starts when the monitor does', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h, { mode: 'continuous' });
    m.markIdle();
    expect(m.isRunning).toBe(false);
    m.start();
    for (let i = 0; i < 100; i++) h.vsync(1000 / 60);
    expect(m.snapshot(true, 0).raf.sinceIdle?.n).toBe(99);
  });
  it('a monitor that was never started reports nothing running', () => {
    const m = new DisplayMonitor(new Fake());
    expect(m.isRunning).toBe(false);
    expect(m.snapshot(true, 0).rafSupported).toBe(false);
  });
});

describe('hzClass', () => {
  it.each([[59.9, 60], [60.2, 60], [74.8, 75], [119.8, 120], [143.9, 144], [165.2, 165], [239, 240]])('%d Hz is the %d Hz class', (hz, c) => { expect(hzClass(hz)).toBe(c); });
  it('nothing near a common rate, or below 50 Hz (throttled), has no class', () => {
    expect(hzClass(49)).toBeNull();
    expect(hzClass(30)).toBeNull();
    expect(hzClass(null)).toBeNull();
    expect(hzClass(110)).toBeNull();
  });
});

describe('hzFit: does the engine\'s rate fit the screen?', () => {
  const raf = (hz: number, peak = hz, n = 600) => ({ recent10s: { n, hz, peakHz: peak } as never, sinceIdle: { n, hz, peakHz: peak } as never, total: { n, hz, peakHz: peak } as never, binEdgesMs: [] });
  const facts = (engineHz: number, envSet = true) => ({ engineEnvHz: envSet ? engineHz : null, engineHz, autoMeasuredHz: null });
  it('120 Hz on a screen that serves 60 frames a second: the screen is the limit', () => {
    const r = hzFit(facts(120), raf(60));
    expect(r.fit).toBe('display-slower');
    expect(r.ratio).toBe(2);
    expect(r.note).toContain('最大 60 fps');
  });
  it('the engine\'s 60 on a 120 Hz screen: the engine does not use the screen', () => {
    const r = hzFit(facts(60, false), raf(120));
    expect(r.fit).toBe('engine-slower');
    expect(r.note).toContain('120 Hz');
  });
  it('120 and 120, 60 and 60 (within 10%) match', () => {
    expect(hzFit(facts(120), raf(119.6)).fit).toBe('match');
    expect(hzFit(facts(60), raf(60.1)).fit).toBe('match');
    expect(hzFit(facts(60), raf(64)).fit).toBe('match');
    expect(hzFit(facts(60), raf(70)).fit).toBe('engine-slower');
  });
  it('a page whose fastest frames are twice as fast as its median is behind, which is not a statement about the screen', () => {
    const r = hzFit(facts(120), raf(60, 120));
    expect(r.fit).toBe('page-behind');
    expect(r.note).toContain('間に合って');
  });
  it('too few intervals is "unknown"', () => {
    expect(hzFit(facts(60), raf(60, 60, 10)).fit).toBe('unknown');
  });
  it('prefers the window since the first idle when it has enough intervals, else the whole run', () => {
    const r = hzFit(facts(60), { recent10s: { n: 600, hz: 60, peakHz: 60 } as never, sinceIdle: { n: 300, hz: 120, peakHz: 120 } as never, total: { n: 900, hz: 80, peakHz: 120 } as never, binEdgesMs: [] });
    expect(r.basis).toBe('since-idle');
    expect(r.rafHz).toBe(120);
    const r2 = hzFit(facts(60), { recent10s: { n: 600, hz: 60, peakHz: 60 } as never, sinceIdle: { n: 10, hz: 30, peakHz: 30 } as never, total: { n: 900, hz: 60, peakHz: 60 } as never, binEdgesMs: [] });
    expect(r2.basis).toBe('total');
  });
});


// ---- decision 0202: the rAF is asked for in bursts, not all the time ----

describe('the monitor in burst mode (decision 0202): no requestAnimationFrame outside a burst', () => {
  class Fake implements DisplayHost {
    t = 0;
    cbs: ((t: number) => void)[] = [];
    asked = 0;
    vis = true;
    visCbs: ((v: boolean) => void)[] = [];
    timers: { id: number; at: number; cb: () => void }[] = [];
    nextTimer = 1;
    now(): number { return T0 + this.t; }
    timeOrigin(): number { return T0; }
    requestFrame(cb: (t: number) => void): void { this.asked++; this.cbs.push(cb); }
    setTimer(cb: () => void, ms: number): unknown { const id = this.nextTimer++; this.timers.push({ id, at: this.t + ms, cb }); return id; }
    clearTimer(handle: unknown): void { this.timers = this.timers.filter((x) => x.id !== handle); }
    isVisible(): boolean { return this.vis; }
    onVisibility(cb: (v: boolean) => void): void { this.visCbs.push(cb); }
    observe(): boolean { return false; }
    vsync(periodMs: number): void {
      this.t += periodMs;
      for (;;) { const due = this.timers.filter((x) => x.at <= this.t).sort((a, b) => a.at - b.at)[0]; if (!due) break; this.timers = this.timers.filter((x) => x !== due); due.cb(); }
      if (!this.vis) return;
      const run = this.cbs.splice(0);
      for (const cb of run) cb(this.t);
    }
    /** `seconds` of 60 Hz screen */
    run(seconds: number): void { for (let i = 0; i < seconds * 60; i++) this.vsync(1000 / 60); }
  }
  const B = 2000, P = 60_000;

  it('the default is burst mode: a burst at the start, one at the first idle, then one every 60 s; between bursts nothing asks for a frame', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h);
    expect(m.mode).toBe('burst');
    m.start();
    expect(m.rafLoopRunning).toBe(true);
    h.run(2.2);                                    // the start burst (2 s) is over
    expect(m.rafLoopRunning).toBe(false);
    const asked = h.asked;
    h.run(6);                                      // 6 quiet seconds: no frame asked for at all
    expect(h.asked).toBe(asked);
    expect(h.cbs).toHaveLength(0);
    m.markIdle();                                  // the first idle: another burst
    expect(m.rafLoopRunning).toBe(true);
    h.run(2.2);
    expect(m.rafLoopRunning).toBe(false);
    const snap = m.snapshot(true, 0);
    expect(snap.mode).toBe('burst');
    expect(snap.burst).toEqual({ onMs: B, periodMs: P, count: 2 });
    expect(snap.rafSupported).toBe(true);
    h.run(58);                                     // 60 s after the first idle: the periodic burst
    h.run(0.1);
    expect(m.snapshot(true, 0).burst?.count).toBe(3);
    expect(m.rafLoopRunning).toBe(true);
    h.run(2.2);
    expect(m.rafLoopRunning).toBe(false);
  });
  it('only the burst\'s own intervals are counted: the gap between two bursts is not a slow frame, and the rate divides by the observed seconds', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h);
    m.start();
    m.markIdle();
    h.run(30);
    const s = m.snapshot(true, 0);
    // 2 s of 60 Hz twice (the start and the first idle are 0 s apart: one burst lasting 2 s from the idle): about 120 intervals, never one of seconds
    expect(s.raf.total.n).toBeGreaterThan(100);
    expect(s.raf.total.n).toBeLessThan(135);
    expect(s.raf.total.maxMs).toBeLessThan(20);
    expect(s.raf.total.slow).toBe(0);
    expect(Math.abs((s.raf.total.hz as number) - 60)).toBeLessThan(0.7);
    expect(Math.abs((s.raf.total.perSecond as number) - 60)).toBeLessThan(1.5);   // callbacks per OBSERVED second, not per second of the page
    expect(Math.abs((s.raf.sinceIdle?.perSecond as number) - 60)).toBeLessThan(1.5);
  });
  it('recent10s is the latest burst, however long ago it was; the next burst replaces it', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h);
    m.start();
    m.markIdle();
    h.run(30);
    const a = m.snapshot(true, 0);
    expect(a.raf.recent10s.n).toBeGreaterThan(100);
    expect(Math.abs((a.raf.recent10s.hz as number) - 60)).toBeLessThan(0.7);
    expect(Math.abs((a.raf.recent10s.perSecond as number) - 60)).toBeLessThan(1.5);
    h.run(40);                                      // the periodic burst at 60 s has come and gone
    const b = m.snapshot(true, 0);
    expect(a.burst?.count).toBe(1);                 // the start and the first idle at the same instant: one burst
    expect(b.burst?.count).toBe(2);
    expect(b.raf.total.n).toBeGreaterThan(a.raf.total.n + 100);
  });
  it('before the first burst there is nothing to read (not zero, null)', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h);
    const s = m.snapshot(true, 0);
    expect(s.rafSupported).toBe(false);
    expect(s.raf.recent10s.n).toBe(0);
    expect(s.raf.recent10s.hz).toBeNull();
  });
  it('a burst already on is made to last 2 s from the first idle (the idle comes in the middle of the start burst)', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h);
    m.start();
    h.run(1.5);
    m.markIdle();
    h.run(1.9);                                     // 3.4 s after the start: the burst was extended to 2 s after the idle (3.5 s)
    expect(m.rafLoopRunning).toBe(true);
    h.run(0.3);
    expect(m.rafLoopRunning).toBe(false);
    expect(m.snapshot(true, 0).burst?.count).toBe(1);   // one burst, not two
  });
  it('a hidden tab during a burst: no ticks, no interval across the hidden stretch', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h);
    m.start();
    h.run(0.5);
    h.vis = false; for (const c of h.visCbs) c(false);
    h.run(1);
    h.vis = true; for (const c of h.visCbs) c(true);
    h.run(2);
    const s = m.snapshot(true, 0);
    expect(s.raf.total.maxMs).toBeLessThan(20);
  });
  it('continuous mode is still a loop all the time (decision 0192\'s behaviour, kept for comparison): a frame asked for at every vsync', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h, { mode: 'continuous' });
    m.start();
    h.run(10);
    expect(m.rafLoopRunning).toBe(true);
    expect(h.asked).toBeGreaterThan(590);
    expect(m.snapshot(true, 0).mode).toBe('continuous');
  });
  it('a burst-mode snapshot passes the beacon shape check; the mode and the burst block are checked', () => {
    const h = new Fake();
    const m = new DisplayMonitor(h);
    m.start(); m.markIdle(); h.run(5);
    const display = JSON.parse(JSON.stringify(m.snapshot(true, 0)));
    expect(beaconProblems({ v: 2, sid: 'a1b2c3d4e5f6', display, hz: null, client: null })).toEqual([]);
    const bad = (mutate: (d: any) => void): string => { const d = JSON.parse(JSON.stringify(display)); mutate(d); return beaconProblems({ v: 2, sid: 'a1b2c3d4e5f6', display: d, hz: null, client: null }).join('\n'); };
    expect(bad((d) => { d.mode = 'sometimes'; })).toContain('display.mode');
    expect(bad((d) => { delete d.burst; })).toContain('display.burst');
    expect(bad((d) => { d.burst.onMs = 90_000; })).toContain('longer than its period');
    expect(bad((d) => { d.burst.count = 1.5; })).toContain('whole number');
    expect(bad((d) => { d.mode = 'continuous'; })).toContain('only a burst-mode record has it');
    const old = JSON.parse(JSON.stringify(display)); delete old.mode; delete old.burst;
    expect(beaconProblems({ v: 2, sid: 'a1b2c3d4e5f6', display: old, hz: null, client: null })).toEqual([]);   // a version 2 record has neither
  });
});
