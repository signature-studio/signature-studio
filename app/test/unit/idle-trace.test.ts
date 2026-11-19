import { describe, expect, it } from 'vitest';
import { IdleTrace } from '../../src/engine/idle-trace.ts';

describe('IdleTrace (decision 0170): a fingerprint of the time the guest lives through', () => {
  const run = (every: number, f: (n: number) => [number, number], count: number) => {
    const t = new IdleTrace(every);
    const out: { n: number; hash: number }[] = [];
    for (let n = 1; n <= count; n++) { const [now, next] = f(n); const c = t.push(n, now, next); if (c) out.push(c); }
    return out;
  };
  const clock = (n: number): [number, number] => [36_000_000_000 + n * 10_000, 36_000_000_000 + (n + 1) * 10_000];

  it('has a checkpoint at every multiple of the interval, none in between', () => {
    expect(run(500, clock, 2000).map((c) => c.n)).toEqual([500, 1000, 1500, 2000]);
  });
  it('the same idles give the same hashes, in a second instance too', () => {
    expect(run(100, clock, 1000)).toEqual(run(100, clock, 1000));
  });
  it('a different clock, deadline or idle number anywhere before a checkpoint changes it, and every later one', () => {
    const base = run(100, clock, 1000);
    const bumpNow = run(100, (n) => (n === 37 ? [clock(n)[0] + 1, clock(n)[1]] : clock(n)), 1000);
    const bumpNext = run(100, (n) => (n === 37 ? [clock(n)[0], clock(n)[1] + 1] : clock(n)), 1000);
    expect(bumpNow[0]?.hash).not.toBe(base[0]?.hash);
    expect(bumpNext[0]?.hash).not.toBe(base[0]?.hash);
    expect(bumpNow.every((c, i) => c.hash !== base[i]?.hash)).toBe(true);
  });
  it('tells apart two clocks that differ only above 32 bits (ticks run past 4.29e9 after about 7 minutes)', () => {
    const a = run(10, (n) => [4_294_967_296 + n, 0], 10);
    const b = run(10, (n) => [2 * 4_294_967_296 + n, 0], 10);
    expect(a[0]?.hash).not.toBe(b[0]?.hash);
  });
});
