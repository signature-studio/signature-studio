import { describe, expect, it } from 'vitest';
import { QuietClock } from '../../src/engine/desk-sleep.ts';

describe('QuietClock: the frame loop sleeps when the picture does not move', () => {
  it('keeps running while a present changes a pixel at least every afterMs', () => {
    const q = new QuietClock(100, 0);
    for (let t = 16; t <= 1000; t += 16) {
      if (t % 64 === 0) q.changed(t);          // a change every 64 ms
      expect(q.sleepAfterFrame(t)).toBe(false);
    }
    expect(q.sleeps).toBe(0);
    expect(q.isAsleep).toBe(false);
  });

  it('goes to sleep after more than afterMs without a change, and not at exactly afterMs', () => {
    const q = new QuietClock(100, 0);
    expect(q.sleepAfterFrame(100)).toBe(false);
    expect(q.sleepAfterFrame(101)).toBe(true);
    expect(q.isAsleep).toBe(true);
    expect(q.sleeps).toBe(1);
  });

  it('a changed present wakes it (true once) and starts the quiet time again', () => {
    const q = new QuietClock(100, 0);
    expect(q.sleepAfterFrame(500)).toBe(true);
    expect(q.changed(520)).toBe(true);        // was asleep: the loop must be started
    expect(q.changed(530)).toBe(false);       // awake: nothing to start
    expect(q.isAsleep).toBe(false);
    expect(q.sleepAfterFrame(600)).toBe(false);   // 70 ms after the change
    expect(q.sleepAfterFrame(631)).toBe(true);
    expect(q.sleeps).toBe(2);
  });

  it('wake() (a canvas that lost its context) restarts a sleeping loop and is a no-op for a running one', () => {
    const q = new QuietClock(100, 0);
    expect(q.wake(10)).toBe(false);
    q.sleepAfterFrame(300);
    expect(q.wake(310)).toBe(true);
    expect(q.isAsleep).toBe(false);
  });
});
