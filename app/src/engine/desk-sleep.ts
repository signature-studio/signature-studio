// Decision 0223 / desk2: when the Worker's frame loop (one requestAnimationFrame per refresh of the display) sleeps. A picture that does not move needs no animation frames: with the refresh rate unlimited the loop ran ~4,000 times a second
// and kept the compositor and the GPU process working for nothing (measured, decision 0223 §12). The loop sleeps after `afterMs` without a present that changed a pixel; the engine's idle (which looks for a change at most every
// DESK_FALLBACK_MS) finds the next change, and the present that shows it wakes the loop. Pure state, no timers, so that a test can drive it with made-up times.

export class QuietClock {
  private readonly afterMs: number;
  private quietSince: number;
  private asleep = false;
  /** how many times the loop went to sleep (a counter for the page's diagnostics) */
  sleeps = 0;

  constructor(afterMs: number, now: number) { this.afterMs = afterMs; this.quietSince = now; }

  get isAsleep(): boolean { return this.asleep; }

  /** a present changed a pixel at `now`; true when the loop was asleep and must be started again */
  changed(now: number): boolean {
    this.quietSince = now;
    const was = this.asleep;
    this.asleep = false;
    return was;
  }

  /** the loop must run again for another reason than a changed present (the canvas lost its context); true when it was asleep */
  wake(now: number): boolean { return this.changed(now); }

  /** after a frame at `now`: true when the loop goes to sleep (nothing changed for more than `afterMs`) */
  sleepAfterFrame(now: number): boolean {
    if (now - this.quietSince > this.afterMs) { this.asleep = true; this.sleeps++; return true; }
    return false;
  }
}
