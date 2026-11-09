// Decision 0272: what the Worker tells the page about the taskbar. The engine's entry (fle_g3_taskbar_iconic) is read at every desktop tick; the page hears of it only when the answer CHANGED, and a build
// without the entry (null) is never heard of. The first answer 0 ("no minimized window") is what the page already assumes, so it is not sent.
export class TaskbarReport {
  private last = 0;

  /** `now`: the engine's answer (null: this build has no taskbar entry). Returns the number to post to the page, or null when there is nothing new to say */
  next(now: number | null): number | null {
    if (now === null || now === this.last) return null;
    this.last = now;
    return now;
  }
}
