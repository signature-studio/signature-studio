import { beforeEach, describe, expect, it } from 'vitest';
import { IN, type RawInput } from '../../src/input/events.ts';
import { IdleDriver, TICKS_PER_MS, type Injected } from '../../src/engine/idle-driver.ts';


// a fake engine (records the calls), a fake host (a clock the test moves, timers it fires by hand)
function rig(opts: { maxQueue?: number; rate?: number; mouseResult?: number } = {}) {
  const calls: string[] = [];
  const delivered: number[] = [];
  const timers: { fn: () => void; ms: number; id: number; live: boolean }[] = [];
  let t = 1000;
  let nextId = 1;
  let waiting = true;
  const injected: Injected[][] = [];
  const processed: { n: number; at: number }[] = [];
  const dropped: [number, string][] = [];
  const idles: (number | null)[] = [];
  const d = new IdleDriver(
    {
      mouse: (k, x, y, data) => { calls.push(`mouse ${k} ${x} ${y} ${data}`); return opts.mouseResult ?? 1; },
      key: (k, vk, sc, e) => { calls.push(`key ${k} ${vk.toString(16)} ${sc.toString(16)} ${e}`); return 1; },
      clockTo: (ticks) => { calls.push(`clock ${ticks}`); },
    },
    {
      deliver: (code) => { delivered.push(code); const w = waiting; waiting = false; return w; },
      now: () => t,
      setTimer: (fn, ms) => { const o = { fn, ms, id: nextId++, live: true }; timers.push(o); return o; },
      clearTimer: (h) => { (h as { live: boolean }).live = false; },
    },
    { injected: (l) => injected.push(l), processed: (l, at) => processed.push({ n: l.length, at }), dropped: (s, r) => dropped.push([s, r]), idle: (_n, _a, _b, busy) => idles.push(busy) },
    { ...opts },
  );
  const idle = (n: number, now: number, next: number): void => { waiting = true; d.onIdle(n, now, next); };
  const live = () => timers.filter((x) => x.live);
  return { d, calls, delivered, timers, live, idle, advance: (ms: number) => { t += ms; }, injected, processed, dropped, idles, setNow: (v: number) => { t = v; } };
}
const move = (x: number, y: number): RawInput => ({ k: 'mouse', kind: IN.MOVE, x, y, data: 0 });
const down = (x: number, y: number): RawInput => ({ k: 'mouse', kind: IN.BDOWN, x, y, data: 0 });
const kd: RawInput = { k: 'key', kind: IN.KDOWN, vk: 0xd, scan: 0x1c, ext: 0 };

describe('IdleDriver: the engine\'s idle in wall-clock mode', () => {

  it('drops input that arrives before the first idle (S-input-before-idle) and says so', () => {
    const r = rig();
    expect(r.d.submit(move(1, 1), 1, 0)).toBe('dropped');
    expect(r.d.stats.droppedBeforeIdle).toBe(1);
    expect(r.dropped).toEqual([[1, expect.stringContaining('first idle')]]);
    expect(r.calls).toEqual([]);
    expect(r.d.ready).toBe(false);
  });

  it('at an idle with a deadline it waits (real time = virtual time at rate 1) and resumes the engine at the deadline, in ticks', () => {
    const r = rig();
    r.idle(1, 36_000_000_000, 36_000_160_000);        // 16 ms away
    expect(r.live()).toHaveLength(1);
    expect(r.live()[0]?.ms).toBeCloseTo(16, 6);
    r.live()[0]?.fn();
    expect(r.delivered).toEqual([36_000_160_000]);    // > 0: move the virtual clock to that many ticks
    expect(r.d.stats.deadlineWakes).toBe(1);
  });

  it('no deadline (next = 0): no timer, the engine waits for input', () => {
    const r = rig();
    r.idle(1, 100, 0);
    expect(r.timers).toHaveLength(0);
  });

  it('a rate other than 1 scales the wait', () => {
    const r = rig({ rate: 2 });
    r.idle(1, 0, 20 * TICKS_PER_MS);
    expect(r.live()[0]?.ms).toBeCloseTo(10, 6);
  });

  it('input at an idle is given at once: the clock moves by the wall time since the idle began, the event is queued in the engine, the engine resumes with 0, the timer is cancelled', () => {
    const r = rig();
    r.idle(1, 1_000_000, 1_000_000 + 100 * TICKS_PER_MS);
    r.advance(30);                                    // 30 ms of wall time in the idle
    expect(r.d.submit(down(958, 599), 7, 5)).toBe('applied');
    expect(r.calls).toEqual([`clock ${1_000_000 + 30 * TICKS_PER_MS}`, 'mouse 2 958 599 0']);
    expect(r.delivered).toEqual([0]);
    expect(r.live()).toHaveLength(0);
    expect(r.d.stats.injected).toBe(1);
    expect(r.injected[0]?.[0]).toMatchObject({ seq: 7, tPage: 5 });
  });

  it('the clock never passes the engine\'s next deadline (it would skip it)', () => {
    const r = rig();
    r.idle(1, 0, 10 * TICKS_PER_MS);
    r.advance(500);
    r.d.submit(move(1, 2), 1, 0);
    expect(r.calls[0]).toBe(`clock ${10 * TICKS_PER_MS}`);
  });

  it('no clock move when no time has passed or the engine has no deadline and no time passed', () => {
    const r = rig();
    r.idle(1, 5, 0);
    r.d.submit(kd, 1, 0);
    expect(r.calls).toEqual(['key 5 d 1c 0']);
  });

  it('input while the engine is busy is queued in order, consecutive moves merged into the last (S-move-merge), and given at the next idle', () => {
    const r = rig();
    r.idle(1, 0, 0);
    r.d.submit(move(0, 0), 1, 0);                      // applied (idle) ...
    expect(r.d.isIdle).toBe(false);                    // ... so the engine is busy now
    expect(r.d.submit(move(1, 1), 2, 0)).toBe('queued');
    expect(r.d.submit(move(2, 2), 3, 0)).toBe('queued');   // merged: replaces seq 2
    expect(r.d.submit(down(2, 2), 4, 0)).toBe('queued');
    expect(r.d.submit(move(3, 3), 5, 0)).toBe('queued');
    expect(r.d.queued).toBe(3);
    expect(r.d.stats.merged).toBe(1);
    expect(r.dropped).toContainEqual([2, 'merged into a later move']);
    r.calls.length = 0;
    r.idle(2, 10, 0);
    expect(r.calls).toEqual(['mouse 1 2 2 0', 'mouse 2 2 2 0', 'mouse 1 3 3 0']);
    expect(r.d.queued).toBe(0);
  });

  it('a move after a button is not merged into the button', () => {
    const r = rig();
    r.idle(1, 0, 0);
    r.d.submit(kd, 1, 0);
    r.d.submit(down(1, 1), 2, 0);
    r.d.submit(move(2, 2), 3, 0);
    expect(r.d.queued).toBe(2);
    expect(r.d.stats.merged).toBe(0);
  });

  it('the queue is bounded: an event beyond it is dropped and counted', () => {
    const r = rig({ maxQueue: 2 });
    r.idle(1, 0, 0);
    r.d.submit(kd, 1, 0);
    r.d.submit(down(1, 1), 2, 0);
    r.d.submit(kd, 3, 0);
    expect(r.d.submit(kd, 4, 0)).toBe('dropped');
    expect(r.d.stats.overflow).toBe(1);
    expect(r.dropped.at(-1)?.[0]).toBe(4);
  });

  it('events given at one idle are reported as processed at the next idle, with that instant', () => {
    const r = rig();
    r.idle(1, 0, 0);
    r.d.submit(kd, 1, 0);
    expect(r.processed).toHaveLength(0);
    r.advance(370);
    r.idle(2, 0, 0);
    expect(r.processed).toEqual([{ n: 1, at: 1370 }]);
    r.idle(3, 0, 0);
    expect(r.processed).toHaveLength(1);              // reported once
  });

  it('how long the engine ran between a resume and the next idle is reported (the frame time)', () => {
    const r = rig();
    r.idle(1, 0, 16 * TICKS_PER_MS);
    r.advance(16);
    r.live()[0]?.fn();                                // resumes at t = 1016
    r.advance(370);
    r.idle(2, 0, 16 * TICKS_PER_MS);
    expect(r.idles).toEqual([null, 370]);
  });

  it('events the engine refuses (its queue is full) are counted as rejected and not reported as processed; the engine is not resumed for them', () => {
    const r = rig({ mouseResult: 0 });
    r.idle(1, 0, 50 * TICKS_PER_MS);
    r.d.submit(move(1, 1), 1, 0);
    expect(r.d.stats.rejected).toBe(1);
    expect(r.delivered).toEqual([]);
    expect(r.live()).toHaveLength(1);                 // the deadline timer is still armed
    expect(r.dropped.at(-1)?.[0]).toBe(1);
  });

  it('end() resumes with a negative code (the session ends)', () => {
    const r = rig();
    r.idle(1, 0, 10);
    expect(r.d.end()).toBe(true);
    expect(r.delivered).toEqual([-1]);
    expect(r.live()).toHaveLength(0);
  });

  it('a timer of an earlier idle that fires late does nothing', () => {
    const r = rig();
    r.idle(1, 0, 10 * TICKS_PER_MS);
    const t1 = r.timers[0];
    r.d.submit(kd, 1, 0);                              // cancels it and resumes
    r.idle(2, 0, 0);
    t1?.fn();
    expect(r.delivered).toEqual([0]);
  });

  it('decision 0280: inputs the runtime took inline are kept as awaiting; the idle that follows answers (processed) the ones not taken away; the clock is not moved', () => {
    const r = rig();
    r.idle(1, 0, 0);
    r.d.injectedInline([{ seq: 5, tPage: 1000, tInject: 1003, ev: down(1, 2) }, { seq: 6, tPage: 1001, tInject: 1003, ev: kd }]);
    expect(r.d.awaitingCount).toBe(2);
    expect(r.calls.filter((c) => c.startsWith('clock'))).toEqual([]);
    expect(r.d.stats.injected).toBe(2);
    expect(r.injected.length).toBe(1);
    const taken = r.d.takeAwaiting();
    expect(taken.map((x) => x.seq)).toEqual([5, 6]);
    expect(r.d.awaitingCount).toBe(0);
    r.d.injectedInline([{ seq: 7, tPage: 1010, tInject: 1012, ev: move(3, 4) }]);
    r.idle(2, 100, 0);
    expect(r.processed).toEqual([{ n: 1, at: 1000 }]);
    expect(r.d.awaitingCount).toBe(0);
  });
});
