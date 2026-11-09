// Decision 0152: the page's own speed numbers, against values worked out by hand (src/perf/stats.ts).
import { describe, expect, it } from 'vitest';
import type { LatencyRecord } from '../../src/ui/latency.ts';
import { inputLatencyStats, intervalStats, LATENCY_LAST, PerfRecorder, RECENT_WINDOW_MS, TARGETS } from '../../src/perf/stats.ts';

describe('intervalStats', () => {
  it('nine updates in the window: gaps 100 200 300 400 7000 500 100 400 -> sorted 100 100 200 300 400 400 500 7000, median (index 3) 300, p95 (index 7) 7000, 9 / 10 s = 0.9 per second', () => {
    const t = [1000, 1100, 1300, 1600, 2000, 9000, 9500, 9600, 10000];
    expect(intervalStats(t, 10000, 0)).toEqual({ n: 9, intervalMedianMs: 300, intervalP95Ms: 7000, perSecond: 0.9 });
  });
  it('only the last 10 s count: at now = 12000 the window is (2000, 12000]; 5500 6000 6500 -> gaps 500 500, 3 / 10 s', () => {
    expect(intervalStats([100, 200, 5500, 6000, 6500], 12000, 100)).toEqual({ n: 3, intervalMedianMs: 500, intervalP95Ms: 500, perSecond: 0.3 });
  });
  it('an update exactly 10 s old is outside the window; one at "now" is inside', () => {
    expect(intervalStats([2000, 3000], 12000, 0).n).toBe(1);
    expect(intervalStats([2000, 3000, 12000], 12000, 0).n).toBe(2);
    expect(intervalStats([12001], 12000, 0).n).toBe(0);
  });
  it('one update has no interval; none has nothing', () => {
    expect(intervalStats([3000], 12000, 0)).toEqual({ n: 1, intervalMedianMs: null, intervalP95Ms: null, perSecond: 0.1 });
    expect(intervalStats([], 12000, 0)).toEqual({ n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: 0 });
  });
  it('a window shorter than 10 s (the engine started 1 s ago): 3 updates / 1 s = 3 per second', () => {
    expect(intervalStats([9100, 9200, 9300], 10000, 9000).perSecond).toBe(3);
  });
  it('no known start, or no time since the start: no rate (not a guess)', () => {
    expect(intervalStats([9100], 10000, null).perSecond).toBeNull();
    expect(intervalStats([9100], 10000, 10000).perSecond).toBeNull();
    expect(intervalStats([9100], 10000, 10500).perSecond).toBeNull();
  });
  it('120 updates per second for 10 s are 120.0 per second with a median gap of 8 ms (gaps 8 8 9 repeated)', () => {
    const t: number[] = [];
    let at = 0;
    for (let i = 0; i < 1200; i++) { at += [8, 8, 9][i % 3] as number; t.push(at); }
    const s = intervalStats(t, 10000, 0);
    expect(s.intervalMedianMs).toBe(8);
    expect(s.intervalP95Ms).toBe(9);
    expect(s.perSecond).toBeCloseTo(t.filter((x) => x > 0 && x <= 10000).length / 10, 9);
  });
  it('p95 is not the maximum once there are 20 or more gaps: 40 updates 1, 3, 6, ... (gaps 2..40 -> 39 gaps) -> p95 = index ceil(0.95 * 39) - 1 = ceil(37.05) - 1 = 37 -> gap 39, the maximum is 40', () => {
    const t: number[] = [0];
    for (let g = 2; g <= 40; g++) t.push((t[t.length - 1] as number) + g);
    const s = intervalStats(t, t[t.length - 1] as number, 0, 100_000);
    expect(s.n).toBe(40);
    expect(s.intervalP95Ms).toBe(39);
    expect(s.intervalMedianMs).toBe(21); // 39 gaps, index ceil(19.5) - 1 = 19 -> gap 21
  });
  it('the constants are the targets of docs/milestones/performance-targets.md', () => {
    expect(RECENT_WINDOW_MS).toBe(10000);
    expect(LATENCY_LAST).toBe(20);
    expect(TARGETS).toEqual({ startupSeconds: 10, updatesPerSecond: 120, inputToUpdateP95Ms: 17 });
  });
});

const rec = (seq: number, frameAfterMs: number | null, over: Partial<LatencyRecord> = {}): LatencyRecord => ({
  seq, desc: 'x', kind: 0, from: 'pointerdown', tPage: 1000 * seq, tEvent: null, tListener: null, tInject: 1000 * seq + 1, tIdle: 1000 * seq + 2,
  held: false, outcome: null, lateFrame: null, frames: frameAfterMs === null ? {} : { 13: 1000 * seq + frameAfterMs }, answers: {}, dropped: null, ...over,
});
const totals = { sent: 30, processed: 25, dropped: 3, pending: 2, answered: 20, late: 2, noAnswer: 3 };

describe('inputLatencyStats', () => {
  it('22 events with input -> update 1..22 ms: the last 20 are 3..22; sorted the same; p50 = index 9 = 12; p95 = index 18 = 21; max 22', () => {
    const records = Array.from({ length: 22 }, (_, i) => rec(i + 1, i + 1));
    const s = inputLatencyStats(records, totals).listenerToPut;
    expect(s.n).toBe(20);
    expect(s.lastMs).toEqual(Array.from({ length: 20 }, (_, i) => i + 3));
    expect(s.p50Ms).toBe(12);
    expect(s.p95Ms).toBe(21);
    expect(s.maxMs).toBe(22);
    const all = inputLatencyStats(records, totals);
    expect({ sent: all.sent, processed: all.processed, dropped: all.dropped, pending: all.pending, answered: all.answered, late: all.late, noAnswer: all.noAnswer }).toEqual(totals);
  });
  it('an event that no frame followed, and a dropped one, are not in the numbers', () => {
    const records = [rec(1, 40), rec(2, null), rec(3, 10, { dropped: 'merged' }), rec(4, 20)];
    const s = inputLatencyStats(records, totals).listenerToPut;
    expect(s.lastMs).toEqual([40, 20]);
    expect(s.n).toBe(2);
    expect(s.p50Ms).toBe(20);
    expect(s.p95Ms).toBe(40);
  });
  it('the first frame of ANY window counts: the earliest of two windows', () => {
    const r = rec(1, 30, { frames: { 13: 1000 + 30, 22: 1000 + 12 } });
    expect(inputLatencyStats([r], totals).listenerToPut.lastMs).toEqual([12]);
  });
  it('nothing measured: nulls and an empty list', () => {
    const empty = { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] };
    expect(inputLatencyStats([], totals)).toMatchObject({ eventToRaf: empty, listenerToPut: empty, stages: null });
  });
  it('the number of kept events can be changed (last = 3)', () => {
    const records = Array.from({ length: 5 }, (_, i) => rec(i + 1, (i + 1) * 10));
    expect(inputLatencyStats(records, totals, 3).listenerToPut.lastMs).toEqual([30, 40, 50]);
  });
});

// decision 0202: a record that has an answer with its animation frame. Event at 1000*seq - 10 (listener +9, page +1), injected +2 later, the Worker posted the answer after `worker` ms, it came back in 3 ms, was put 4 ms later, the next animation frame started `raf` ms after that.
const answered = (seq: number, worker: number, raf: number): LatencyRecord => {
  const tEvent = 1000 * seq - 10, tListener = tEvent + 9, tPage = tListener + 1, tInject = tPage + 2;
  const postedAt = tInject + worker, receivedAt = postedAt + 3, at = receivedAt + 4;
  return { ...rec(seq, null), tPage, tEvent, tListener, tInject, tIdle: tInject + worker + 1, frames: { 13: at }, answers: { 13: { lid: 13, at, postedAt, receivedAt, raf: at + raf } } };
};

describe('inputLatencyStats: inputs without a value (decision 0205)', () => {
  const lateOne = (seq: number, raf: number): LatencyRecord => ({ ...rec(seq, null), tEvent: 1000 * seq - 10, outcome: 'late', lateFrame: { lid: 13, at: 1000 * seq + 40, postedAt: 1000 * seq + 35, receivedAt: 1000 * seq + 38, raf: 1000 * seq + 40 + raf } });
  it('an input that is late has no eventToRaf and no listenerToPut (it is not in the answered p95); an input with no frame at all is in no statistic', () => {
    const records = [answered(1, 3, 2), lateOne(2, 6), lateOne(3, 10), rec(4, null, { outcome: 'none' })];
    const s = inputLatencyStats(records, totals);
    expect(s.eventToRaf.n).toBe(1);
    expect(s.listenerToPut.n).toBe(1);
  });
  it('without dedupe the page cannot tell a changed frame from a redraw: lateVisible and eventToRafWithLate are null, never an empty statistic', () => {
    const s = inputLatencyStats([answered(1, 3, 2), lateOne(2, 6)], totals);
    expect(s.lateVisible).toBeNull();
    expect(s.eventToRafWithLate).toBeNull();
  });
  it('with ?dedupe=1 the late inputs\' first later frame is a visible change: lateVisible holds them (event -> rAF), eventToRafWithLate holds the answered ones and the late ones in the order of the inputs; eventToRaf is untouched', () => {
    const a1 = answered(1, 3, 2), a4 = answered(4, 3, 2);
    const records = [a1, lateOne(2, 6), lateOne(3, 10), a4, rec(5, null, { outcome: 'none' })];
    const s = inputLatencyStats(records, totals, 20, true);
    expect(s.lateVisible!.lastMs).toEqual([56, 60]);   // 10 + 40 + raf
    expect(s.lateVisible!.p50Ms).toBe(56);
    expect(s.lateVisible!.maxMs).toBe(60);
    expect(s.eventToRaf.n).toBe(2);
    const ev = s.eventToRaf.lastMs;
    expect(s.eventToRafWithLate!.lastMs).toEqual([ev[0], 56, 60, ev[1]]);
    expect(s.eventToRafWithLate!.n).toBe(4);
    expect(s.eventToRafWithLate!.maxMs).toBe(Math.max(60, ...ev));
  });
  it('the late visible changes can move the p95 the answered inputs alone would give: 19 fast answered inputs and 3 late ones at 80 ms', () => {
    const records: LatencyRecord[] = [];
    for (let i = 1; i <= 19; i++) records.push(answered(i, 1, 1));   // 9 + 1 + 2 + 1 + 3 + 4 + 1 = 21 ms each
    for (let i = 20; i <= 22; i++) records.push(lateOne(i, 40));     // 10 + 40 + 40 = 90 ms
    const s = inputLatencyStats(records, totals, 20, true);
    expect(s.eventToRaf.p95Ms).toBe(21);
    expect(s.eventToRafWithLate!.p95Ms).toBe(90);
  });
});

describe('inputLatencyStats: the P2 number from the event\'s timeStamp to the next animation frame, the old number beside it (decision 0202)', () => {
  it('22 events: eventToRaf is event.timeStamp -> rAF (= 9 + 1 + 2 + worker + 3 + 4 + raf), the last 20, its own p50 / p95 / max; listenerToPut is the old number over the same events', () => {
    const records = Array.from({ length: 22 }, (_, i) => answered(i + 1, i + 1, 2));
    const s = inputLatencyStats(records, totals);
    expect(s.eventToRaf.lastMs).toEqual(Array.from({ length: 20 }, (_, i) => 9 + 1 + 2 + (i + 3) + 3 + 4 + 2));
    expect(s.eventToRaf.n).toBe(20);
    expect(s.eventToRaf.p50Ms).toBe(33);   // sorted: 24..43, index 9 -> 24 + 9
    expect(s.eventToRaf.p95Ms).toBe(42);   // index 18
    expect(s.eventToRaf.maxMs).toBe(43);
    // the old number: tPage -> put = 2 + worker + 3 + 4
    expect(s.listenerToPut.lastMs).toEqual(Array.from({ length: 20 }, (_, i) => 2 + (i + 3) + 3 + 4));
    expect(s.listenerToPut.p95Ms).toBe(30);
    for (let i = 0; i < 20; i++) expect((s.eventToRaf.lastMs[i] as number) - (s.listenerToPut.lastMs[i] as number)).toBe(9 + 1 + 2);   // S0 + S0b + S4a = 12: the part the old number left out
  });
  it('the stages are the medians and 95th percentiles over the same events; they are the pieces of the sum', () => {
    const records = Array.from({ length: 5 }, (_, i) => answered(i + 1, 10 * (i + 1), 2 + i));
    const s = inputLatencyStats(records, totals);
    expect(s.stages).toEqual({
      n: 5,
      p50: { s0: 9, s0b: 1, toWorker: 2, worker: 30, fromWorker: 3, put: 4, raf: 4 },
      p95: { s0: 9, s0b: 1, toWorker: 2, worker: 50, fromWorker: 3, put: 4, raf: 6 },
    });
  });
  it('an event without a timeStamp, one with no animation frame yet, and a dropped one are not in eventToRaf; the first still has the old number', () => {
    const noStamp = { ...answered(1, 5, 2), tEvent: null, tListener: null };
    const noRaf = answered(2, 5, 2);
    noRaf.answers[13]!.raf = null;
    const lost = answered(3, 5, 2);
    lost.answers[13]!.raf = -1;
    const dropped = { ...answered(4, 5, 2), dropped: 'merged' };
    const s = inputLatencyStats([noStamp, noRaf, lost, dropped, answered(5, 5, 2)], totals);
    expect(s.eventToRaf.n).toBe(1);
    expect(s.listenerToPut.n).toBe(4);   // all but the dropped one
    expect(s.stages!.n).toBe(1);
  });
  it('nothing has an animation frame: eventToRaf and stages are empty, not zero', () => {
    const s = inputLatencyStats([rec(1, 20)], totals);
    expect(s.eventToRaf).toEqual({ n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] });
    expect(s.listenerToPut.p50Ms).toBe(20);
    expect(s.stages).toBeNull();
  });
});

describe('PerfRecorder', () => {
  it('counts every update and idle, keeps the rate from the engine start, and reports the window ending now', () => {
    const r = new PerfRecorder();
    r.start(1000);
    for (const t of [1500, 2500, 3500]) r.update(t);
    r.idle(2000); r.idle(4000);
    const s = r.snapshot(5000);
    expect(s.observedSince).toBe(1000);
    expect(s.updates.total).toBe(3);
    expect(s.updates.recent).toEqual({ n: 3, intervalMedianMs: 1000, intervalP95Ms: 1000, perSecond: 0.75 });
    expect(s.idles.total).toBe(2);
    expect(s.idles.recent).toEqual({ n: 2, intervalMedianMs: 2000, intervalP95Ms: 2000, perSecond: 0.5 });
  });
  it('an idle report can stand for several idles (decision 0170): the total is exact, the times are those of the reports', () => {
    const r = new PerfRecorder();
    r.start(0);
    r.idle(1000, 40); r.idle(2000, 1000); r.idle(3000);
    const s = r.snapshot(4000);
    expect(s.idles.total).toBe(1041);
    expect(s.idles.recent.n).toBe(3);
  });
  it('without start() the rate counts from the first update', () => {
    const r = new PerfRecorder();
    r.update(1000); r.update(2000);
    expect(r.snapshot(3000).observedSince).toBe(1000);
    expect(r.snapshot(3000).updates.recent.perSecond).toBe(1);
  });
  it('its memory is bounded, the total is not', () => {
    const r = new PerfRecorder(100);
    for (let i = 1; i <= 1000; i++) r.update(i);
    const s = r.snapshot(1000);
    expect(s.updates.total).toBe(1000);
    expect(s.updates.recent.n).toBeLessThanOrEqual(200);
    expect(s.updates.recent.n).toBeGreaterThanOrEqual(100);
  });
});
