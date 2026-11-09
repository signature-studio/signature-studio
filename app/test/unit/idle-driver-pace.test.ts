import { describe, expect, it } from 'vitest';
import { IN, type RawInput } from '../../src/input/events.ts';
import { IdleDriver, TICKS_PER_MS, type DriverHost, type DriverOptions } from '../../src/engine/idle-driver.ts';

// Decision 0170: the virtual clock follows the wall clock. A fake engine (it reports a deadline `gapMs` of virtual time ahead at every idle and spends `cycleMs` of wall time per cycle), a fake wall clock and a fake
// event loop (timers that fire no sooner than `clampMs` [the nested-timer clamp of the browser] and `lateMs` after they were asked for; bounces that run at once but only in a later turn). Microtasks are the engine's resumes:
// they run to exhaustion after every task, like the real engine's JSPI resumes, which is why the driver has a turn budget.
interface Task { at: number; seq: number; fn: () => void; live: boolean; kind: 'timer' | 'bounce' }

function sim(opts: DriverOptions & { gapMs?: number; cycleMs?: number | ((n: number) => number); clampMs?: number; lateMs?: number } = {}) {
  let wall = 1_000_000;
  let virt = 36_000_000_000;
  const virt0 = virt, wall0 = wall;
  const gap = (opts.gapMs ?? 1) * TICKS_PER_MS;
  let idleNo = 0;
  let waiting = false;
  let seq = 0;
  const tasks: Task[] = [];
  const resumes: number[] = [];
  const codes: number[] = [];
  const clockTo: number[] = [];
  const calls: string[] = [];
  const nows: number[] = [];
  const count = { timers: 0, bounces: 0 };
  const order: string[] = [];
  const cuts: number[] = [];
  let freeze = 0;
  let freezeAdvance = 0;
  let chain = 0, maxChain = 0, turns = 0;
  const cycleOf = typeof opts.cycleMs === 'function' ? opts.cycleMs : (): number => (opts.cycleMs as number | undefined) ?? 0.1;
  const host: DriverHost = {
    deliver: (code) => { if (!waiting) return false; waiting = false; resumes.push(code); codes.push(code); return true; },
    now: () => wall,
    setTimer: (fn, ms) => { count.timers++; const t: Task = { at: wall + Math.max(ms, opts.clampMs ?? 0) + (opts.lateMs ?? 0), seq: seq++, fn, live: true, kind: 'timer' }; tasks.push(t); return t; },
    clearTimer: (h) => { (h as Task).live = false; },
    yieldTurn: (fn) => { count.bounces++; tasks.push({ at: wall, seq: seq++, fn, live: true, kind: 'bounce' }); },
  };
  const d = new IdleDriver(
    {
      mouse: (k, x, y, data) => { calls.push(`mouse ${k} ${x} ${y} ${data} @${virt}`); return 1; },
      key: (k, vk) => { calls.push(`key ${k} ${vk} @${virt}`); return 1; },
      clockTo: (t) => { clockTo.push(t); if (t > virt) virt = t; },
    },
    host,
    { idle: () => { order.push('idle'); }, resumed: (_c, _a, why) => { order.push(`resume:${why}`); }, lagCut: (t) => { cuts.push(t); } },
    opts,
  );
  const cycle = (): void => {
    const cost = freeze > 0 ? freeze : cycleOf(idleNo + 1);
    freeze = 0;
    wall += cost;
    virt += freezeAdvance * TICKS_PER_MS;
    freezeAdvance = 0;
    waiting = true;
    nows.push(virt);
    d.onIdle(++idleNo, virt, virt + gap);
  };
  const drain = (): void => {
    let guard = 0;
    while (resumes.length) {
      if (++guard > 5_000_000) throw new Error('runaway');
      const code = resumes.shift() as number;
      if (code > 0 && code > virt) virt = code;
      chain++;
      cycle();
    }
    if (chain > maxChain) maxChain = chain;
    chain = 0;
  };
  const turn = (): void => { turns++; };
  return {
    d, calls, codes, clockTo, count, host, nows, order, cuts,
    start: () => { cycle(); drain(); },
    /** the next cycle of the engine takes `ms` of wall time (a stall) */
    stall: (ms: number, advanceVirtualMs = 0) => { freeze = ms; freezeAdvance = advanceVirtualMs; },
    run: (ms: number) => {
      const end = wall + ms;
      for (;;) {
        const live = tasks.filter((t) => t.live).sort((a, b) => a.at - b.at || a.seq - b.seq);
        const t = live[0];
        if (!t || t.at > end) break;
        tasks.splice(tasks.indexOf(t), 1);
        if (t.at > wall) wall = t.at;
        turn();
        t.live = false;
        t.fn();
        drain();
      }
      if (wall < end) wall = end;
    },
    advance: (ms: number) => { wall += ms; },
    submit: (ev: RawInput) => { const r = d.submit(ev, 1, wall); drain(); return r; },
    end: () => { const r = d.end(); drain(); return r; },
    virtualMs: () => (virt - virt0) / TICKS_PER_MS,
    wallMs: () => wall - wall0,
    virt: () => virt,
    maxChain: () => maxChain,
    turns: () => turns,
    live: () => tasks.filter((t) => t.live),
  };
}
const kd: RawInput = { k: 'key', kind: IN.KDOWN, vk: 0xd, scan: 0x1c, ext: 0 };

describe('IdleDriver pace: the virtual clock follows the wall clock (decision 0170)', () => {
  it('catch-up: an engine that reports a 1 ms deadline per cycle behind timers that the browser clamps to 4 ms: the virtual clock stays the virtual clock within a few ms of the wall clock', () => {
    const s = sim({ clampMs: 4, cycleMs: 0.11 });
    s.start();
    s.run(4000);
    expect(s.virtualMs() / s.wallMs()).toBeGreaterThan(0.99);
    expect(s.wallMs() - s.virtualMs()).toBeLessThan(6);
    expect(s.virtualMs()).toBeLessThanOrEqual(s.wallMs() + 1e-6);          // never ahead of the wall clock
    expect(s.d.stats.chained).toBeGreaterThan(s.d.stats.timerWakes);       // most deadlines are delivered in the turn of the idle, not by a timer
  });

  it('every deadline is delivered exactly as the engine reported it: no virtual time is skipped, in order, whatever the lag', () => {
    const s = sim({ cycleMs: 0.3, gapMs: 1 });
    s.start();
    s.stall(700);
    s.run(1500);
    const g = TICKS_PER_MS;
    for (let i = 1; i < s.nows.length; i++) expect((s.nows[i] as number) - (s.nows[i - 1] as number)).toBe(g);   // the engine saw every virtual ms
    expect(s.codes.every((c, i) => i === 0 || c === (s.codes[i - 1] as number) + g)).toBe(true);
  });

  it('a stall of the engine (a lag of 700 ms) is caught up without timers: the due deadlines come one after the other in the same turn, then in bounces; the lag is not cut under the cap', () => {
    const s = sim({ cycleMs: 0.5, gapMs: 1 });
    s.start();
    s.run(50);
    s.stall(700);
    s.run(10);                                                // the stall happens in the next cycle
    expect(s.wallMs() - s.virtualMs()).toBeGreaterThan(600);
    const v0 = s.virtualMs(), w0 = s.wallMs();
    const timers0 = s.count.timers;
    for (let i = 0; i < 4000 && s.wallMs() - s.virtualMs() > 3; i++) s.run(1);
    expect(s.wallMs() - s.virtualMs()).toBeLessThanOrEqual(3);   // caught up ...
    expect(s.count.timers - timers0).toBeLessThanOrEqual(2);     // ... without timers (a wake per deadline is what made the clock slow)
    expect(s.d.stats.lagCuts).toBe(0);
    expect(s.d.stats.bounces).toBeGreaterThan(10);
    s.run(500);
    expect((s.virtualMs() - v0) / (s.wallMs() - w0)).toBeGreaterThan(1.4);   // faster than real time while it caught up
  });

  it('turn budget: a catch-up gives the event loop a turn at least every turnBudgetMs of wall time (input, timers, paint get through)', () => {
    const s = sim({ cycleMs: 0.5, turnBudgetMs: 4 });
    s.start();
    s.stall(400);
    s.run(1000);
    expect(s.maxChain()).toBeLessThanOrEqual(Math.ceil(4 / 0.5) + 2);
    expect(s.d.stats.bounces).toBeGreaterThan(40);
  });

  it('turn budget 0: every deadline is delivered in a turn of its own', () => {
    const s = sim({ cycleMs: 1.2, turnBudgetMs: 0 });          // the engine needs more wall time than the 1 ms it advances: it is always behind
    s.start();
    s.run(100);
    expect(s.d.stats.chained).toBe(0);
    expect(s.maxChain()).toBe(1);
    expect(s.d.stats.bounces).toBeGreaterThan(50);
  });

  it('a lag beyond maxLagMs is cut once by moving the anchor: the virtual clock stays behind for good, nothing is skipped, the cut is reported', () => {
    const s = sim({ cycleMs: 0.1, maxLagMs: 1000 });
    s.start();
    s.run(20);
    s.stall(5000);
    s.run(20);
    s.run(3000);
    expect(s.d.stats.lagCuts).toBe(1);
    expect(s.cuts).toHaveLength(1);                            // the hook said so once, with the amount that was cut
    expect(s.cuts[0]).toBe(s.d.stats.lagCutTicks);
    expect(s.d.stats.lagCutTicks / TICKS_PER_MS).toBeGreaterThan(3900);
    expect(s.d.stats.lagCutTicks / TICKS_PER_MS).toBeLessThan(4100);
    const behind = s.wallMs() - s.virtualMs();
    expect(behind).toBeGreaterThan(3900);                      // about the part of the 5 s that was cut, plus what the cycles cost
    expect(behind).toBeLessThan(4100);
    const g = TICKS_PER_MS;
    for (let i = 1; i < s.nows.length; i++) expect((s.nows[i] as number) - (s.nows[i - 1] as number)).toBe(g);
    expect(s.d.stats.maxLagTicks / TICKS_PER_MS).toBeLessThanOrEqual(1000.0001);
  });

  it('a tab whose timers are throttled to one a second: each wake catches up what it can (up to the cap), the average speed stays 1', () => {
    const s = sim({ cycleMs: 0.1, lateMs: 984, maxLagMs: 1000 });   // a 16 ms wait fires about a second late
    s.start();
    s.run(20000);
    expect(s.d.stats.lagCuts).toBe(0);
    expect(s.virtualMs() / s.wallMs()).toBeGreaterThan(0.93);
  });

  it('...and when the throttle is longer than the cap (3 s late) the excess is cut at each wake: slower than real time, but never a skipped ms', () => {
    const s = sim({ cycleMs: 0.1, lateMs: 3000, maxLagMs: 1000 });
    s.start();
    s.run(30000);
    expect(s.d.stats.lagCuts).toBeGreaterThan(3);
    expect(s.virtualMs() / s.wallMs()).toBeLessThan(0.5);
    const g = TICKS_PER_MS;
    for (let i = 1; i < s.nows.length; i++) expect((s.nows[i] as number) - (s.nows[i - 1] as number)).toBe(g);
  });

  it('a lead (the engine\'s own computation moved its clock ahead of the wall clock) is kept up to maxLeadMs: the next wait is stretched by it, so the long-run speed stays 1', () => {
    const s = sim({ cycleMs: 0.1, gapMs: 16 });
    s.start();
    s.run(100);
    s.stall(10, 40);                                           // a stretch of 10 ms of wall time in which the engine\'s clock moved 40 ms
    for (let i = 0; i < 40 && s.d.stats.maxLeadTicks === 0; i++) s.run(1);   // the timer that was armed fires, the stretch happens
    const t = s.live()[0];
    expect(s.d.stats.leadCuts).toBe(0);
    expect(s.d.stats.maxLeadTicks / TICKS_PER_MS).toBeGreaterThan(25);
    expect(t).toBeDefined();
    s.run(2000);
    expect(s.virtualMs() / s.wallMs()).toBeGreaterThan(0.99);
    expect(s.virtualMs() / s.wallMs()).toBeLessThan(1.01);
  });

  it('a lead beyond maxLeadMs is cut: the wait never exceeds the virtual distance to the deadline by more than the cap', () => {
    const s = sim({ cycleMs: 0.1, gapMs: 16, maxLeadMs: 100 });
    s.start();
    s.run(100);
    s.stall(10, 400);
    for (let i = 0; i < 40 && s.d.stats.leadCuts === 0; i++) s.run(1);
    expect(s.d.stats.leadCuts).toBe(1);
    const t = s.live()[0] as { at: number };
    expect(t.at - (1_000_000 + s.wallMs())).toBeLessThanOrEqual(16 + 100 + 1);
    expect(t.at - (1_000_000 + s.wallMs())).toBeGreaterThan(100);
  });

  it('an input given while a due deadline waits for its bounce is applied at once: the clock moves to the deadline, the event is queued, the engine resumes with 0, and the bounce does nothing', () => {
    const s = sim({ cycleMs: 0.5, turnBudgetMs: 4 });
    s.start();
    s.stall(100);
    s.run(1);                                                  // the stall; the engine is behind by 100 ms and is waiting for a bounce
    s.run(0);
    expect(s.d.isIdle).toBe(true);
    const before = s.codes.length;
    const v = s.virt();
    expect(s.submit(kd)).toBe('applied');
    expect(s.clockTo.at(-1)).toBe(v + TICKS_PER_MS);           // moved to the deadline the engine reported (1 ms ahead), not past it, although the wall clock is 100 ms ahead
    expect(s.calls.at(-1)).toMatch(/^key /);
    expect(s.codes[before]).toBe(0);
    s.run(5);                                                  // the bounce that was pending runs now and must not deliver the deadline a second time
    for (let i = 1; i < s.nows.length; i++) expect((s.nows[i] as number) - (s.nows[i - 1] as number)).toBe(TICKS_PER_MS);
  });

  it('input while the engine is idle waiting for a future deadline: the clock moves by the wall time that passed, never past the deadline (as before)', () => {
    const s = sim({ cycleMs: 0.1, gapMs: 100 });
    s.start();                                                 // the engine reports a deadline 100 ms ahead; the driver waits
    expect(s.live().length).toBe(1);
    s.advance(30);
    const v0 = s.virt();
    s.submit(kd);
    expect(s.clockTo.at(-1)).toBe(v0 + 30 * TICKS_PER_MS);
    s.advance(500);
  });

  it('end() resumes with -1, and a bounce that was pending does nothing afterwards', () => {
    const s = sim({ cycleMs: 0.5, turnBudgetMs: 0 });
    s.start();
    expect(s.live().length).toBe(1);
    s.end();
    expect(s.codes.at(-1)).toBe(-1);
    s.run(10);
    expect(s.codes.filter((c) => c === -1)).toHaveLength(1);
  });

  it('no deadline (next = 0): nothing is delivered and no timer is armed', () => {
    const calls: number[] = [];
    let t = 5000;
    const d = new IdleDriver({ mouse: () => 1, key: () => 1, clockTo: () => undefined }, { deliver: (c) => { calls.push(c); return true; }, now: () => t, setTimer: () => { throw new Error('no timer expected'); }, clearTimer: () => undefined });
    d.onIdle(1, 100, 0);
    t += 100000;
    d.onIdle(2, 100, 0);
    expect(calls).toEqual([]);
  });

  it('rate 2: virtual time runs twice as fast as the wall clock', () => {
    const s = sim({ cycleMs: 0.05, rate: 2, clampMs: 4 });
    s.start();
    s.run(2000);
    expect(s.virtualMs() / s.wallMs()).toBeGreaterThan(1.97);
    expect(s.virtualMs() / s.wallMs()).toBeLessThan(2.03);
  });

  it('without yieldTurn the bounce goes through setTimer(fn, 0)', () => {
    const timers: [() => void, number][] = [];
    let t = 0;
    const d = new IdleDriver({ mouse: () => 1, key: () => 1, clockTo: () => undefined }, { deliver: () => true, now: () => t, setTimer: (fn, ms) => { timers.push([fn, ms]); return timers.length; }, clearTimer: () => undefined }, {}, { turnBudgetMs: 0 });
    d.onIdle(1, 0, 10000);
    t += 5;                                                    // the deadline (1 ms) is 4 ms past
    d.onIdle(2, 0, 10000);
    expect(timers.at(-1)?.[1]).toBe(0);
    });

  it('an idle is reported to the hooks before the resume that follows it at once, in every cycle (the page must see idle, then resume)', () => {
    const s = sim({ cycleMs: 1.2, turnBudgetMs: 4 });          // always behind: every deadline is due at once
    s.start();
    s.run(30);
    expect(s.order.length).toBeGreaterThan(20);
    s.order.forEach((e, i) => { expect(e).toBe(i % 2 === 0 ? 'idle' : 'resume:deadline'); });
  });

  it('a guest whose own cycle is 120 Hz (a heavy cycle every 8 virtual ms, light ones between) is followed as well as one at 60 Hz, and the host uses no more turns than idles', () => {
    const run = (period: number, heavyMs: number) => {
      const s = sim({ cycleMs: (n) => (n % period === 0 ? heavyMs : 0.11), gapMs: 1 });
      s.start();
      s.run(3000);
      return s;
    };
    for (const [period, heavy] of [[16, 2.0], [8, 2.0], [8, 5.0]] as const) {
      const s = run(period, heavy);
      expect(s.virtualMs() / s.wallMs()).toBeGreaterThan(0.99);
      expect(s.d.stats.lagCuts).toBe(0);
      expect(s.turns()).toBeLessThan(s.d.stats.idleCount);
    }
  });
});
