import { describe, expect, it } from 'vitest';
import { ReportThrottle } from '../../src/engine/report-throttle.ts';

describe('ReportThrottle (decision 0170): what the Worker tells the page about the engine\'s idles and resumes', () => {
  it('the first idle is reported at once; later idles at most once per gap', () => {
    const t = new ReportThrottle(40);
    expect(t.idle(1, 1000)).toBe(true);
    expect(t.idle(2, 1001)).toBe(false);
    expect(t.idle(3, 1039.9)).toBe(false);
    expect(t.idle(4, 1040)).toBe(true);
    expect(t.reportedN).toBe(4);
    expect(t.idle(5, 1041)).toBe(false);
  });

  it('a resume after a reported idle is reported (so a long stretch of computing is announced); a deadline resume after a reported resume is not', () => {
    const t = new ReportThrottle(40);
    t.idle(1, 0);
    expect(t.resume('deadline')).toBe(true);
    expect(t.idle(2, 1)).toBe(false);             // not reported: the page still says "computing"
    expect(t.resume('deadline')).toBe(false);
    expect(t.resume('deadline')).toBe(false);
    expect(t.idle(3, 50)).toBe(true);
    expect(t.resume('deadline')).toBe(true);
  });

  it('a resume for an input event or the end of the session is always reported', () => {
    const t = new ReportThrottle(40);
    t.idle(1, 0);
    t.resume('deadline');
    expect(t.resume('input')).toBe(true);
    expect(t.resume('end')).toBe(true);
  });

  it('a stretch of computing longer than the gap always ends in a reported idle (the page learns the engine is idle again)', () => {
    const t = new ReportThrottle(40);
    t.idle(1, 0);
    t.resume('deadline');
    for (let n = 2; n < 20; n++) t.idle(n, n);   // short cycles, not reported
    expect(t.idle(20, 4000)).toBe(true);
  });

  it('a gap of 0 reports every idle', () => {
    const t = new ReportThrottle(0);
    expect([1, 2, 3].map((n) => t.idle(n, 5))).toEqual([true, true, true]);
  });
});
