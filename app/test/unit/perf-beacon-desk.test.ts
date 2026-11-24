// Decision 0240 (gpu1): the two optional measurement blocks of the beacon that say how the desktop was presented and how the engine ran (read-only: they only report counters the page already has).
import { describe, expect, it } from 'vitest';
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildBeacon, deskBeacon, deskCounters, DESK_HISTORY_MAX, DeskHistory, deskRecent, engineRunBeacon, type BeaconInput } from '../../src/perf/beacon.ts';
import { beaconProblems } from '../../shared/perf.ts';

const stats2d = { presents: 100, rects: 120, pixels: 5_000_000, composeMs: 321.46, putMs: 98.76, whole: 1, ticks: 400, idleTicks: 30, wakes: 12, sleeps: 3 };
const statsGl = { ...stats2d, drawMs: 12.34, uploads: 118, draws: 100, texAllocs: 2, uploadBytes: 20_000_000 };

describe('deskCounters / deskBeacon', () => {
  it('a 2D presenter has no draw counters; a GL presenter has them and says so', () => {
    expect(deskBeacon(stats2d, null, 1000)).toMatchObject({ mode: '2d', presents: 100, rects: 120, mpx: 5, composeMs: 321.5, putMs: 98.8, drawMs: 0, whole: 1, since: null });
    expect(deskBeacon(statsGl, null, 1000)).toMatchObject({ mode: 'gl', uploads: 118, draws: 100, drawMs: 12.3 });
  });
  it('no counters, no block', () => {
    expect(deskBeacon(null, null, 1000)).toBeNull();
    expect(deskCounters(undefined)).toBeNull();
  });
  it('since the previous beacon: the differences and the seconds between them', () => {
    const prev = { stats: { ...stats2d, presents: 40, rects: 20, pixels: 1_000_000, composeMs: 100, putMs: 50 }, atMs: 10_000 };
    const d = deskBeacon(stats2d, prev, 40_000);
    expect(d?.since).toMatchObject({ seconds: 30, presents: 60, mpx: 4, composeMs: 221.5, putMs: 48.8, rects: 100 });
  });
  it('a value that is not a finite number counts as 0', () => {
    expect(deskCounters({ presents: 'x', pixels: NaN, composeMs: Infinity })?.presents).toBe(0);
  });
});

describe('engineRunBeacon', () => {
  const idle = (at: number, busyMs: number | null, nowMs: number): { at: number; busyMs: number | null; now: number } => ({ at, busyMs, now: nowMs * 10_000 });
  it('the busy share of the wall time, the median and p95 of the run lengths and the virtual clock per wall ms', () => {
    const idles = [idle(0, null, 0), idle(100, 20, 50), idle(200, 20, 100), idle(300, 40, 150), idle(400, 100, 200)];
    expect(engineRunBeacon(idles, 400)).toEqual({ windowSeconds: 0.4, idles: 5, busyFraction: 0.45, busyMsMedian: 40, busyMsP95: 100, virtualPerWall: 0.5 });
  });
  it('only the last 10 s count; fewer than 2 idles, or no wall time: no block', () => {
    expect(engineRunBeacon([idle(0, 5, 0), idle(30_000, 5, 20_000), idle(35_000, 5, 25_000)], 35_000)?.idles).toBe(2);
    expect(engineRunBeacon([idle(100, 5, 0)], 200)).toBeNull();
    expect(engineRunBeacon([idle(100, 5, 0), idle(100, 5, 0)], 200)).toBeNull();
  });
});

describe('the presents of the last 10 s (Issue #1398)', () => {
  it('a step count: the presents from the window\'s start to now, over the seconds the window covers', () => {
    const h = new DeskHistory();
    for (let t = 0; t <= 30_000; t += 100) h.add(t, t / 100); // 10 presents a second
    expect(deskRecent(h.samples, 30_000)).toEqual({ seconds: 10, presents: 100, presentsPerSec: 10 });
    expect(h.samples.length).toBeLessThan(DESK_HISTORY_MAX);
  });
  it('a history that starts less than 10 s ago counts from its first report, and says over how many seconds', () => {
    const h = new DeskHistory();
    h.add(5000, 50); h.add(6000, 60); h.add(7000, 70);
    expect(deskRecent(h.samples, 7000)).toEqual({ seconds: 2, presents: 20, presentsPerSec: 10 });
  });
  it('the count is a step: between two reports it holds; a report after now does not count', () => {
    const s = [{ at: 0, presents: 0 }, { at: 9000, presents: 90 }, { at: 20_000, presents: 500 }];
    expect(deskRecent(s, 15_000)).toEqual({ seconds: 10, presents: 90, presentsPerSec: 9 });
  });
  it('under a second there is no rate; no report at all is no count', () => {
    expect(deskRecent([{ at: 1000, presents: 5 }], 1500)).toEqual({ seconds: 0.5, presents: 0, presentsPerSec: null });
    expect(deskRecent([], 1000)).toEqual({ seconds: 0, presents: 0, presentsPerSec: null });
  });
  it('the history keeps the last 10 s and the report that is the window\'s base, and is capped', () => {
    const h = new DeskHistory();
    for (let t = 0; t <= 60_000; t += 100) h.add(t, t / 100);
    expect(h.samples[0]?.at).toBe(50_000);
    const capped = new DeskHistory();
    for (let i = 0; i < DESK_HISTORY_MAX + 300; i++) capped.add(i, i);
    expect(capped.samples).toHaveLength(DESK_HISTORY_MAX);
  });
  it('a count that goes down is a new presenter: the history starts again', () => {
    const h = new DeskHistory();
    h.add(0, 100); h.add(1000, 200); h.add(2000, 3);
    expect(h.samples).toEqual([{ at: 2000, presents: 3 }]);
  });
  it('deskBeacon carries recent10s from the history it is given; without one it says there are none', () => {
    const h = new DeskHistory();
    for (let t = 0; t <= 12_000; t += 100) h.add(t, t / 100);
    expect(deskBeacon(statsGl, null, 12_000, h.samples)?.recent10s).toEqual({ seconds: 10, presents: 100, presentsPerSec: 10 });
    expect(deskBeacon(stats2d, null, 1000)?.recent10s).toEqual({ seconds: 0, presents: 0, presentsPerSec: null });
  });
});

describe('the fps fixtures (Issue #1398): records the summary table shows', () => {
  const dir = fileURLToPath(new URL('../../../tools/perf-beacons/fixtures/fps', import.meta.url));
  it('two records, the version PerfFpsFixture, the shape check passes, and the new items are there', () => {
    const files = readdirSync(dir).filter((f) => f.endsWith('.json'));
    expect(files).toHaveLength(2);
    for (const f of files) {
      const rec = JSON.parse(readFileSync(join(dir, f), 'utf8'));
      expect(rec.perf.bundle.version).toBe('PerfFpsFixture');
      expect(beaconProblems(rec.perf)).toEqual([]);
      expect(rec.perf.desk.recent10s.presentsPerSec).toEqual(expect.any(Number));
      expect(rec.perf.rafIntervalMsP95).toBe(rec.perf.display.raf.recent10s.p95Ms);
      expect(rec.perf.input.inputToPresentMsP50).toBe(rec.perf.input.eventToRaf.p50Ms);
    }
  });
  it('the summary table shows the version and the medians over the two records', () => {
    const summary = fileURLToPath(new URL('../../../tools/perf-beacons/summary.mjs', import.meta.url));
    const out = execFileSync(process.execPath, [summary, dir, '--day', 'all'], { encoding: 'utf8' });
    expect(out).toContain('PerfFpsFixture');
    expect(out).toContain('89 (2 記録)');
  });
});

describe('buildBeacon carries the blocks only when it is given them', () => {
  const base = { seq: 1, reason: 'periodic', pageMs: 1, visibility: 'visible', browser: { userAgent: 'u', webglRenderer: null }, bundle: { name: 'b', version: 'v', engineKind: 'emscripten-fruity-v1' }, present: 'D', state: 'engine-idle', exit: null, error: null, timeline: { idle: 1000 }, updates: { total: 0, recent: { n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: 0 } }, idles: { total: 0, recent: { n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: 0 } }, input: { sent: 0, processed: 0, dropped: 0, pending: 0, eventToRaf: { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] }, listenerToPut: { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] }, stages: { n: 0, p50: {}, p95: {} } }, underruns: null } as unknown as BeaconInput;
  it('absent without them (a page from before this change, the stand-in), present with them', () => {
    expect('desk' in buildBeacon(base)).toBe(false);
    expect('engineRun' in buildBeacon(base)).toBe(false);
    const b = buildBeacon({ ...base, desk: deskBeacon(statsGl, null, 0), engineRun: { windowSeconds: 10, idles: 200, busyFraction: 0.5, busyMsMedian: 3, busyMsP95: 9, virtualPerWall: 0.4 } });
    expect(b.desk?.mode).toBe('gl');
    expect(b.engineRun?.virtualPerWall).toBe(0.4);
    expect(b.v).toBe(3);
  });
});
