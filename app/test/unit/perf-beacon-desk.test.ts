// Decision 0240 (gpu1): the two optional measurement blocks of the beacon that say how the desktop was presented and how the engine ran (read-only: they only report counters the page already has).
import { describe, expect, it } from 'vitest';
import { buildBeacon, deskBeacon, deskCounters, engineRunBeacon, type BeaconInput } from '../../src/perf/beacon.ts';

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
