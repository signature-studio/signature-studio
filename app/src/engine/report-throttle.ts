// What the Worker tells the page about the engine's idles and resumes (decision 0170). With the virtual clock following the wall clock the engine idles about 1,000 times a second (one idle per virtual ms the guest
// asks for), and a message per idle and per resume made the page's main thread spend 6-8% of a core on them. The page needs the first idle at once, then the engine's state to about a frame's precision:
//   idle    reported the first time, then at most once per `minGapMs` (a cycle that ends a stretch longer than the gap is always reported, because the gap has passed);
//   resume  reported when the page believes the engine is idle (the last report was an idle), so a long stretch of computing is always announced; a resume that follows a resume is not (the page already says "computing");
//           a resume for an input event or the end of the session is always reported.
// The page learns how many idles it did not see from the idle number `n`. Pure: the Worker passes the instants.
export class ReportThrottle {
  private lastIdleAt = -Infinity;
  private pageSaysIdle = true;
  private lastIdleN = 0;
  private readonly minGapMs: number;
  constructor(minGapMs = 40) { this.minGapMs = minGapMs; }

  /** should the idle number `n` at instant `at` (ms) be reported? */
  idle(n: number, at: number): boolean {
    if (n !== 1 && at - this.lastIdleAt < this.minGapMs) return false;
    this.lastIdleAt = at;
    this.lastIdleN = n;
    this.pageSaysIdle = true;
    return true;
  }

  resume(why: 'input' | 'deadline' | 'end'): boolean {
    if (why === 'deadline' && !this.pageSaysIdle) return false;
    this.pageSaysIdle = false;
    return true;
  }

  /** the idle number of the last reported idle (0: none yet) */
  get reportedN(): number { return this.lastIdleN; }
}
