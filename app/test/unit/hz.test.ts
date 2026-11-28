import { describe, expect, it } from 'vitest';
import { HZ_DEFAULT, HZ_WINDOW_MS, STANDARD_HZ, applyHz, hzEnv, measureFrameRate, roundToStandardHz, type FrameHost } from '../../src/engine/hz.ts';

// a fake frame clock: `intervals(i)` is the time from frame i to frame i+1; `stopAfter` frames are delivered, then requestAnimationFrame goes quiet (a hidden tab); the timer fires when the fake time passes it
function fakeHost(intervals: (i: number) => number, stopAfter = Infinity): { host: FrameHost; time: () => number; timers: number } {
  let t = 1000, i = 0, frameCb: ((t: number) => void) | null = null, timerFn: (() => void) | null = null, timerAt = Infinity, timers = 0, cleared = 0;
  const host: FrameHost = {
    frame(cb) { frameCb = cb; },
    setTimer(fn, ms) { timerFn = fn; timerAt = t + ms; timers++; return 1; },
    clearTimer() { cleared++; timerFn = null; timerAt = Infinity; },
  };
  // drive: called by the test after measureFrameRate() to run the clock until the promise settles
  (host as unknown as { run: () => void }).run = () => {
    for (let guard = 0; guard < 100000 && frameCb; guard++) {
      if (i >= stopAfter) { t = timerAt; const f = timerFn; timerFn = null; frameCb = null; f?.(); return; }
      const cb = frameCb; frameCb = null;
      cb(t);
      t += intervals(i++);
      if (timerFn && t >= timerAt) { const f = timerFn; timerFn = null; f(); return; }
    }
    if (timerFn) { const f = timerFn; timerFn = null; f(); }   // the clock stood still: the give-up time comes by itself
  };
  return { host, time: () => t, get timers() { return timers - cleared; } };
}
const run = (h: FrameHost): void => (h as unknown as { run: () => void }).run();

describe('the engine environment (decision 0182)', () => {
  it('carries the SAME value for both variables, as decimal text', () => {
    expect(hzEnv(120)).toEqual({ FLE_DWM_HZ: '120', FLE_DISPLAYSETTINGS_FREQUENCY: '120' });
  });
});

describe('the display rate: rounding a measured rate to the nearest standard rate (S-hz-standard, S-hz-round)', () => {
  it('the standard list spans 30 Hz (Issue #1402) through 300 Hz', () => { expect(STANDARD_HZ).toEqual([30, 60, 75, 90, 120, 144, 165, 180, 240, 300]); });
  it.each([
    [60, 60], [59.94, 60], [61, 60], [75, 75], [74.9, 75], [90, 90], [100, 90], [119.88, 120], [120, 120], [143.9, 144], [144, 144], [165, 165], [180, 180], [239.76, 240], [240, 240],
    [30, 30], [29.9, 30], [31, 30], [24, 30], [48, 60], [50, 60], [300, 300], [360, 300], [1000, 300],
    [67, 60], [68, 75], [82, 75], [83, 90], [103, 90], [105, 120], [131, 120], [132, 144], [154, 144], [155, 165], [172, 165], [173, 180], [207, 180], [208, 240], [268, 240], [269, 300],
  ])('%f Hz -> %i', (m, want) => { expect(roundToStandardHz(m)).toBe(want); });
  it('no measured rate gives more than 300, and every rate at or above 300 gives 300', () => {
    for (let m = 1; m <= 2000; m += 0.5) expect(roundToStandardHz(m)).toBeLessThanOrEqual(300);
    for (const m of [300, 360, 500, 1000]) expect(roundToStandardHz(m)).toBe(300);
  });
  it('nonsense is the default', () => {
    expect(roundToStandardHz(0)).toBe(HZ_DEFAULT);
    expect(roundToStandardHz(-5)).toBe(HZ_DEFAULT);
    expect(roundToStandardHz(NaN)).toBe(HZ_DEFAULT);
    expect(roundToStandardHz(Infinity)).toBe(HZ_DEFAULT);
  });
});

describe('measureFrameRate (a fake frame clock)', () => {
  it.each([[1000 / 60, 60], [1000 / 120, 120], [1000 / 144, 144], [1000 / 165, 165], [1000 / 75, 75], [1000 / 180, 180], [1000 / 240, 240], [1000 / 300, 300], [1000 / 30, 30]])('steady frames %f ms -> %i Hz', async (iv, want) => {
    const f = fakeHost(() => iv);
    const p = measureFrameRate(f.host);
    run(f.host);
    const m = await p;
    expect(m.hz).toBe(want);
    expect(m.measuredHz).not.toBeNull();
    expect(m.measuredHz as number).toBeCloseTo(1000 / iv, 6);
    expect(m.intervals).toBeGreaterThanOrEqual(20);
    expect(f.timers).toBe(0); // the give-up timer is cleared
  });
  it('the window is covered (HZ_WINDOW_MS = 500 ms, decision 0243; it was 1000): at 60 Hz about 30 intervals after the skipped ones, at 240 Hz about 120', async () => {
    expect(HZ_WINDOW_MS).toBe(500);
    const a = fakeHost(() => 1000 / 60); const pa = measureFrameRate(a.host); run(a.host); const ma = await pa;
    const b = fakeHost(() => 1000 / 240); const pb = measureFrameRate(b.host); run(b.host); const mb = await pb;
    expect(ma.intervals).toBeGreaterThanOrEqual(29); expect(ma.intervals).toBeLessThanOrEqual(32);
    expect(mb.intervals).toBeGreaterThanOrEqual(119); expect(mb.intervals).toBeLessThanOrEqual(122);
    expect(ma.hz).toBe(60); expect(mb.hz).toBe(240);
  });
  it('late frames of the page\'s start-up do not move the answer (median): every 7th frame is 5x late, the first intervals are slow', async () => {
    const f = fakeHost((i) => (i < 5 ? 80 : i % 7 === 0 ? 5 * (1000 / 120) : 1000 / 120));
    const p = measureFrameRate(f.host);
    run(f.host);
    expect((await p).hz).toBe(120);
  });
  it('jitter around 144 Hz selects 144', async () => {
    let s = 7;
    const rnd = (): number => { s = (s * 1103515245 + 12345) & 0x7fffffff; return s / 0x7fffffff; };
    const f = fakeHost(() => (1000 / 144) * (0.95 + 0.1 * rnd()));
    const p = measureFrameRate(f.host);
    run(f.host);
    expect((await p).hz).toBe(144);
  });
  it('a browser that stops animation frames (a hidden tab) gives the default after the give-up time, and says so', async () => {
    const f = fakeHost(() => 1000 / 60, 3);
    const p = measureFrameRate(f.host);
    run(f.host);
    const m = await p;
    expect(m.hz).toBe(HZ_DEFAULT);
    expect(m.measuredHz).toBeNull();
    expect(m.note).toMatch(/only \d+ frame intervals/);
  });
  it('a throttled page (5 frames a second) gives the default: the window never fills with enough intervals before the give-up time', async () => {
    const f = fakeHost(() => 200);
    const p = measureFrameRate(f.host);
    run(f.host);
    const m = await p;
    expect(m.measuredHz).toBeNull();
    expect(m.hz).toBe(HZ_DEFAULT);
  });
  it('equal timestamps (a median interval of 0) are not a rate', async () => {
    const f = fakeHost(() => 0);
    const p = measureFrameRate(f.host, { giveUpMs: 50 });
    run(f.host);
    const m = await p;
    expect(m.measuredHz).toBeNull();
    expect(m.hz).toBe(HZ_DEFAULT);
  });
  it('the options are honoured: a shorter window and fewer intervals answer sooner', async () => {
    const f = fakeHost(() => 1000 / 120);
    const p = measureFrameRate(f.host, { skipIntervals: 1, windowMs: 100, minIntervals: 5 });
    run(f.host);
    const m = await p;
    expect(m.hz).toBe(120);
    expect(m.intervals).toBeLessThan(20);
  });
});

describe('applyHz: the measured rate goes into the engine environment (decision 0182)', () => {
  const base = (): Record<string, string> => ({ FLE_CALL_FRUITY: '1', FLE_VFS_SEED: '/data/seed/x.tsv' });
  it('the measured rate: both variables, the same value, nothing else changed; the log says how it was measured', () => {
    const env = base();
    const r = applyHz({ measuredHz: 143.7, intervals: 144, hz: 120, note: '144 intervals, median 6.96 ms = 143.7 Hz, rounded to 120 Hz' }, env);
    expect(env).toEqual({ ...base(), FLE_DWM_HZ: '120', FLE_DISPLAYSETTINGS_FREQUENCY: '120' });
    expect(r.hz).toBe(120);
    expect(r.warning).toBeNull();
    expect(r.lines[0]?.text).toContain('rounded to 120 Hz');
    expect(r.lines[0]?.level).toBe('info');
  });
  it('an unmeasurable page is 60 Hz with a warning line', () => {
    const env = base();
    const r = applyHz({ measuredHz: null, intervals: 3, hz: 60, note: 'only 3 frame intervals were seen' }, env);
    expect(env.FLE_DWM_HZ).toBe('60');
    expect(env.FLE_DISPLAYSETTINGS_FREQUENCY).toBe('60');
    expect(r.lines[0]?.level).toBe('warn');
  });
  it('no measurement at all is 60 Hz with a warning line', () => {
    const env = base();
    const r = applyHz(null, env);
    expect(r.hz).toBe(60);
    expect(env.FLE_DWM_HZ).toBe('60');
    expect(r.lines[0]?.level).toBe('warn');
  });
});
