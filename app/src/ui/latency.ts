// Latency of an input event (decision 0141), measured on one clock (performance.timeOrigin + performance.now(), ms): the page handles the DOM event (tPage) -> the engine Worker gives it to the engine (tInject: the
// engine is idle, so this is when the Worker's event loop got to the message) -> the engine has processed it and is idle again (tIdle) -> the page has drawn a frame of a window (frames[lid]).
// The engine's idle returns only between its frames (about 0.37 s of computing each in steady state: decision 0138), so tIdle - tInject is the time the guest needed to process the event and draw.
// Decision 0202: two definitions of 'input -> update' are kept side by side. listenerToPut (version 2's, `toAnyFrame`) starts inside the page's listener (tPage) and ends at the putImageData of the answer's frame; it misses the browser's wait before
// it delivers the event (S0) and its wait after the draw for the next frame (S4a), and read 10-25 ms below what a person feels (decision 0194). eventToRaf (`toEventRaf`) starts at the DOM event's own timeStamp (`tEvent`, the instant the
// browser made the event) and ends at the first requestAnimationFrame callback that starts after the answer's putImageData. Both use the same answer: the first frame the Worker posted at or after the injection (decision 0170).
// Decision 0205: the answer is narrowed. It is the first frame the Worker posted between the injection (tInject) and the engine's idle that ends the processing (tIdle, plus IN_WINDOW_MARGIN_MS) - the frame that shows what the guest
// drew for this event. Decision 0170 took the first frame posted at or after the injection, whenever that was: (which sends no frame whose pixels are unchanged) an input that changes nothing on the screen (a button released,
// a move over a place that does not react) was 'answered' by the next unrelated frame, 0.1-0.4 s later. An input with no frame in the window has no answer (null, left out of the percentiles) and is COUNTED (answered / late / noAnswer).
// A press, and a move while a button is held, may be answered by a later repaint (lat1, decision 0194: a knob's first press is drawn 42-47 ms after the injection, in a later idle): for these the first frame within STREAM_MS of the injection is the answer.
// Any other input whose first frame comes within LATE_MS after the window is 'late' (counted apart, no value).
import { describeRaw, IN, type RawInput } from '../input/events.ts';

/** what the page knows of the DOM event that made an input (all on the clock performance.timeOrigin + performance.now()); absent for a cause that is not an input event ('blur', 'test') */
export interface InputMeta {
  /** event.timeStamp: when the browser made the event */
  tEvent: number;
  /** when the page's listener started */
  tListener: number;
}

/** a frame drawn on the page, kept for the events that may be answered by it (see LatencyTracker.frame) */
export interface SeenFrame {
  lid: number;
  /** the putImageData calls returned (page clock) */
  at: number;
  /** the Worker's clock */
  postedAt: number;
  /** the page's message handler started (page clock) */
  receivedAt: number;
  /** the start of the first requestAnimationFrame callback after `at`; null: not yet; -1: there was none soon enough (the tab was hidden) or no rAF */
  raf: number | null;
}

/** how the engine answered an input (decision 0205). window: a frame posted between the injection and the engine's idle; stream: no such frame, but the input is a press or a move with a button down and a frame was posted within STREAM_MS of the injection;
 *  late: no answer, the first frame came within LATE_MS after the window; none: no frame soon enough; null: not decided yet (the input has not been processed, or the wait is not over) */
export type Outcome = 'window' | 'stream' | 'late' | 'none' | null;

export interface LatencyRecord {
  seq: number;
  desc: string;
  kind: number;
  /** the DOM event (or other cause: 'blur', 'test') that made the page send it */
  from: string;
  tPage: number;
  /** decision 0202: event.timeStamp of the DOM event and the start of its listener (null: the cause is not an input event, or its timeStamp is not on this clock) */
  tEvent: number | null;
  tListener: number | null;
  tInject: number | null;
  tIdle: number | null;
  /** decision 0205: a button press, or a move while a button is down (lat1's 'stream': it may be answered by a later repaint) */
  held: boolean;
  outcome: Outcome;
  /** decision 0205: for a 'late' input, the first frame that came within LATE_MS after the window (not an answer: it may be a delayed repaint for this input or an unrelated frame) */
  lateFrame: SeenFrame | null;
  /** the first frame drawn on the page for each window (lid) that is an answer (decision 0205: posted between tInject and tIdle + IN_WINDOW_MARGIN_MS, or for a held input within STREAM_MS of tInject) */
  frames: Record<number, number>;
  /** decision 0202: the frames above with what is known of each (the same ones, by window) */
  answers: Record<number, SeenFrame>;
  dropped: string | null;
}

/** an event.timeStamp that is later than the listener, or more than this much earlier, is not on the page's clock (an old browser, a coarsened clock): the event has no tEvent */
export const EVENT_STAMP_MAX_AGE_MS = 10_000;
/** a requestAnimationFrame callback later than this after the draw is not 'the next frame' (the tab was hidden): the answer has no eventToRaf */
export const RAF_MAX_WAIT_MS = 2000;
/** decision 0205 (S-dd2-1: lat1's IN_MARGIN): the engine's idle ends the processing; the frames drawn in that idle are posted just before the instant tIdle is taken, so a frame up to this much after tIdle still belongs to it */
export const IN_WINDOW_MARGIN_MS = 1;
/** decision 0205 (S-dd2-2: lat1's LATE_MS): a frame this soon after the window is counted as a 'late' answer (no value) */
export const LATE_MS = 150;
/** decision 0205 (S-dd2-3: lat1's stream rule): a press or a move with a button down is answered by the first frame within this long of its injection, even in a later idle */
export const STREAM_MS = 500;

/** decision 0205: the page knows that every frame it gets changed pixels only when the Worker leaves out the unchanged rectangles ; dedupe off sends the redraws too */
export const framesAreChangesFor = (dedupe: boolean | 'check'): boolean => dedupe === true;

export interface LatencyOptions {
  /** decision 0205: every frame the page receives changed pixels (sends no rectangle whose pixels are unchanged), so a 'late' input's later frame is a visible change; false (default, ?dedupe off or check): the page cannot tell */
  framesAreChanges?: boolean;
  /** schedule `cb` for the next animation frame (default: requestAnimationFrame when there is one) */
  raf?: ((cb: () => void) => void) | null;
  /** the page clock, ms (default: performance.timeOrigin + performance.now()) */
  now?: () => number;
}

export function percentile(sorted: readonly number[], p: number): number | null {
  if (sorted.length === 0) return null;
  const i = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1));
  return sorted[i] as number;
}

const SEEN_KEEP = 512;

export class LatencyTracker {
  readonly records: LatencyRecord[] = [];
  private readonly bySeq = new Map<number, LatencyRecord>();
  private sentCount = 0;
  private doneCount = 0;
  private droppedCount = 0;
  /** decision 0205: processed inputs by how they were answered (the counters never shrink when the records are trimmed) */
  private answeredCount = 0;
  private lateCount = 0;
  private noAnswerCount = 0;
  /** processed inputs whose outcome is not decided (a frame may still come within the wait) */
  private readonly waiting: LatencyRecord[] = [];
  /** the mouse buttons the page has told the engine are down (a press, or a move while one is down, is a 'held' input) */
  private readonly buttons = new Set<number>();
  private lastProcessed: LatencyRecord | null = null;
  private readonly keep: number;
  /** the frames drawn lately (see frame()) */
  private readonly seen: SeenFrame[] = [];
  private readonly rafImpl: ((cb: () => void) => void) | null;
  private readonly clock: () => number;
  private rafArmed = false;
  /** see LatencyOptions.framesAreChanges */
  /** decision 0223: set when the display method is known (D shows a present only when a pixel changed) */
  framesAreChanges: boolean;
  constructor(keep = 400, opts: LatencyOptions = {}) {
    this.keep = keep;
    this.framesAreChanges = opts.framesAreChanges === true;
    this.rafImpl = opts.raf !== undefined ? opts.raf : typeof requestAnimationFrame === 'function' ? (cb) => { requestAnimationFrame(() => cb()); } : null;
    this.clock = opts.now ?? (() => performance.timeOrigin + performance.now());
  }

  sent(seq: number, tPage: number, ev: RawInput, from = '', meta: InputMeta | null = null): void {
    // an event.timeStamp is used only when it can be on the page's clock: not after the listener started, and not long before the page sent the event
    const usable = meta !== null && Number.isFinite(meta.tEvent) && Number.isFinite(meta.tListener) && meta.tEvent <= meta.tListener && tPage - meta.tEvent <= EVENT_STAMP_MAX_AGE_MS;
    let held = false;
    if (ev.k === 'mouse') {
      if (ev.kind === IN.BDOWN) { this.buttons.add(ev.data); held = true; }
      else if (ev.kind === IN.BUP) this.buttons.delete(ev.data);
      else if (ev.kind === IN.MOVE) held = this.buttons.size > 0;
    }
    const r: LatencyRecord = {
      seq, desc: describeRaw(ev), kind: ev.kind, from, tPage, tEvent: usable ? (meta as InputMeta).tEvent : null, tListener: usable ? Math.min((meta as InputMeta).tListener, tPage) : null,
      tInject: null, tIdle: null, held, outcome: null, lateFrame: null, frames: {}, answers: {}, dropped: null,
    };
    this.records.push(r);
    this.bySeq.set(seq, r);
    this.sentCount++;
    if (this.records.length > this.keep) { const old = this.records.shift(); if (old) { this.bySeq.delete(old.seq); this.decide(old, 'none'); } }
  }

  done(items: readonly { seq: number; tPage: number; tInject: number; tIdle: number }[]): void {
    for (const it of items) {
      const r = this.bySeq.get(it.seq);
      if (!r || r.tIdle !== null) continue;
      r.tInject = it.tInject;
      r.tIdle = it.tIdle;
      this.creditSeen(r);
      this.doneCount++;
      this.lastProcessed = r;
      if (Object.keys(r.answers).length > 0) this.decide(r, 'window');
      else {
        this.waiting.push(r);
        for (const f of this.seen) if (this.lateFrame(r, f) !== null) break;
      }
    }
  }

  dropped(seq: number, reason: string): void {
    const r = this.bySeq.get(seq);
    if (!r || r.dropped !== null || r.tIdle !== null) return;
    r.dropped = reason;
    this.droppedCount++;
  }

  /**
   * A frame was drawn on the page for window `lid` at `at` (the putImageData calls returned; the page's message handler started at `receivedAt`); the Worker posted it at `postedAt` (the Worker's clock, which is also the clock of `tInject` and `tIdle`;
   * default: `at`).
   * Decision 0205: it answers an event if it was posted between the event's injection and the engine's idle that ended its processing (tIdle + IN_WINDOW_MARGIN_MS), for the first frame of each window. Decision 0170: the Worker draws what the
   * engine drew in the idle that ends an event and posts the frame BEFORE it posts that event's `input-done` (the page learns `tInject` and `tIdle` from `input-done`), so the answer to an event arrives while the record has no `tInject` yet:
   * the frames seen so far are kept, and `done()` credits the ones posted in the window. A frame that arrives after `input-done` is outside the window (unless it was posted within the margin), so it is not an answer; it can still answer an input that
   * is waiting for a later repaint (held, see STREAM_MS), or make an input 'late' (LATE_MS). Without the window, the next unrelated frame (a blinking caret, half a second later) was taken for the answer to an input that changed nothing.
   * Decision 0202: when the frame answers an event (or may still answer one that has not been injected yet), one requestAnimationFrame is asked for, and the start of its callback closes the answer's eventToRaf. Nothing is asked for
   * while no event is waiting, so an idle page costs nothing.
   */
  frame(lid: number, at: number, postedAt: number = at, receivedAt: number = at): void {
    const f: SeenFrame = { lid, at, postedAt, receivedAt, raf: null };
    this.seen.push(f);
    if (this.seen.length > SEEN_KEEP) this.seen.shift();
    let credited = false;
    for (let i = this.records.length - 1; i >= 0; i--) {
      const r = this.records[i] as LatencyRecord;
      if (r.tInject === null || r.tIdle === null) continue;
      if (postedAt < r.tInject) continue;
      if (postedAt - r.tInject > STREAM_MS + LATE_MS) break;
      if (postedAt <= r.tIdle + IN_WINDOW_MARGIN_MS && (r.outcome === null || r.outcome === 'window')) {
        if (r.frames[lid] === undefined) { r.frames[lid] = at; r.answers[lid] = f; credited = true; }
        if (r.outcome === null) this.decide(r, 'window');
      }
    }
    for (const r of [...this.waiting]) { const o = this.lateFrame(r, f); if (o === 'stream' || o === 'late') credited = true; }
    this.expire();
    if (credited || this.mayBeAnswered(at)) this.armRaf();
  }

  /**
   * Decision 0223 (display method D): the picture is made by the Worker AFTER the engine is idle again (the compositor reads the windows the engine drew in answer to the input), so the present that shows an input's
   * result can never be "posted between the injection and the idle" (decision 0205's window, which was written for a frame the engine posts while it processes). The Worker says which inputs a present answers: the
   * ones it had just processed in the turn that made the present, when the present changed a pixel (a present that changes nothing is not made: the compositor reports only what differs). Those inputs are answered
   * by that frame, at `at` (the present returned). An input whose result did not change a pixel gets no frame: unanswered (the compositor reports only what differs).
   */
  answer(seqs: readonly number[], lid: number, at: number): void {
    const f: SeenFrame = { lid, at, postedAt: at, receivedAt: at, raf: null };
    let credited = false;
    for (const seq of seqs) {
      const r = this.bySeq.get(seq);
      if (!r || r.tInject === null || r.tIdle === null || r.outcome === 'window' || r.outcome === 'stream') continue;
      if (r.frames[lid] === undefined) { r.frames[lid] = at; r.answers[lid] = f; }
      this.setAnswered(r);
      credited = true;
    }
    if (credited) {
      this.seen.push(f);
      if (this.seen.length > SEEN_KEEP) this.seen.shift();
      this.armRaf();
    }
  }

  /** an input answered by a frame: counted once, whatever it was before (an input that was 'late' or unanswered is moved) */
  private setAnswered(r: LatencyRecord): void {
    if (r.outcome === 'late') this.lateCount--;
    else if (r.outcome === 'none') this.noAnswerCount--;
    r.outcome = 'window';
    this.answeredCount++;
    const i = this.waiting.indexOf(r);
    if (i >= 0) this.waiting.splice(i, 1);
  }

  /** the frame `f` could settle the processed input `r` that has no answer yet: it is the answer of a held input, makes another input 'late', or is too late for either. Returns what it decided, null: nothing (the frame is not after the window) */
  private lateFrame(r: LatencyRecord, f: SeenFrame): Exclude<Outcome, null> | null {
    if (r.tInject === null || r.tIdle === null || r.outcome !== null) return null;
    const end = r.tIdle + IN_WINDOW_MARGIN_MS;
    if (f.postedAt < r.tInject || f.postedAt <= end) return null;
    let o: Exclude<Outcome, null> = 'none';
    if (r.held && f.postedAt <= r.tInject + STREAM_MS) {
      r.frames[f.lid] = f.at;
      r.answers[f.lid] = f;
      o = 'stream';
    } else if (!r.held && f.postedAt <= end + LATE_MS) { o = 'late'; r.lateFrame = f; }
    this.decide(r, o);
    return o;
  }

  private creditSeen(r: LatencyRecord): void {
    const tInject = r.tInject as number, end = (r.tIdle as number) + IN_WINDOW_MARGIN_MS;
    for (const f of this.seen) {
      if (f.postedAt < tInject || f.postedAt > end) continue;
      if (r.frames[f.lid] === undefined) { r.frames[f.lid] = f.at; r.answers[f.lid] = f; }
    }
  }

  private decide(r: LatencyRecord, outcome: Exclude<Outcome, null>): void {
    if (r.outcome !== null) return;
    if (r.tIdle === null) { r.outcome = outcome; return; }          // trimmed before it was processed: not counted
    r.outcome = outcome;
    if (outcome === 'window' || outcome === 'stream') this.answeredCount++;
    else if (outcome === 'late') this.lateCount++;
    else this.noAnswerCount++;
    const i = this.waiting.indexOf(r);
    if (i >= 0) this.waiting.splice(i, 1);
  }

  /** a waiting input whose wait is over (by the clock; no frame came) has no answer */
  private expire(): void {
    if (this.waiting.length === 0) return;
    const now = this.clock();
    for (const r of [...this.waiting]) {
      if (r.outcome !== null || r.tIdle === null || r.tInject === null) continue;
      const until = r.held ? r.tInject + STREAM_MS : r.tIdle + IN_WINDOW_MARGIN_MS + LATE_MS;
      if (now > until) this.decide(r, 'none');
    }
  }

  /** an event sent lately and not injected yet: the frame that was just drawn may turn out to be its answer */
  private mayBeAnswered(at: number): boolean {
    if (this.pending <= 0) return false;
    for (let i = this.records.length - 1; i >= 0; i--) {
      const r = this.records[i] as LatencyRecord;
      if (at - r.tPage > 5000) return false;
      if (r.tIdle === null && r.dropped === null) return true;
    }
    return false;
  }

  private armRaf(): void {
    if (this.rafArmed || this.rafImpl === null) return;
    this.rafArmed = true;
    this.rafImpl(() => this.onRaf());
  }

  private onRaf(): void {
    this.rafArmed = false;
    const t = this.clock();
    for (let i = this.seen.length - 1; i >= 0; i--) {
      const f = this.seen[i] as SeenFrame;
      if (f.raf !== null) break;
      f.raf = t >= f.at && t - f.at <= RAF_MAX_WAIT_MS ? t : -1;
    }
  }

  /** decision 0205: the processed inputs by how they were answered, as of now (a waiting input whose wait is over counts as unanswered). processed = answered + late + noAnswer + awaiting. */
  get outcomes(): { answered: number; late: number; noAnswer: number; awaiting: number } {
    this.expire();
    return { answered: this.answeredCount, late: this.lateCount, noAnswer: this.noAnswerCount, awaiting: this.waiting.length };
  }
  /** the counters of the beacon's `input` block (perf/stats.ts InputLatencyStats) */
  totals(): { sent: number; processed: number; dropped: number; pending: number; answered: number; late: number; noAnswer: number } {
    const o = this.outcomes;
    return { sent: this.sentCount, processed: this.doneCount, dropped: this.droppedCount, pending: this.pending, answered: o.answered, late: o.late, noAnswer: o.noAnswer };
  }

  /** events sent and neither processed nor dropped (waiting in the Worker's message queue, its own queue or the engine's) */
  get pending(): number { return this.sentCount - this.doneCount - this.droppedCount; }
  get sentTotal(): number { return this.sentCount; }
  get processedTotal(): number { return this.doneCount; }
  get droppedTotal(): number { return this.droppedCount; }
  get last(): LatencyRecord | null { return this.lastProcessed; }

  /** p50 / p95 / max of `pick(record)` over the records for which it gives a number */
  summary(pick: (r: LatencyRecord) => number | null, filter: (r: LatencyRecord) => boolean = () => true): { n: number; p50: number | null; p95: number | null; max: number | null } {
    const v = this.records.filter((r) => r.dropped === null && filter(r)).map(pick).filter((x): x is number => x !== null).sort((a, b) => a - b);
    return { n: v.length, p50: percentile(v, 50), p95: percentile(v, 95), max: v.length ? (v[v.length - 1] as number) : null };
  }
}

/** event handled by the page -> engine idle again */
export const toProcessed = (r: LatencyRecord): number | null => (r.tIdle === null ? null : r.tIdle - r.tPage);
/** event handled by the page -> first frame of window `lid` drawn on the page */
export const toFrame = (lid: number) => (r: LatencyRecord): number | null => (r.frames[lid] === undefined ? null : (r.frames[lid] as number) - r.tPage);
/** the first frame of ANY window: version 2's 'input -> update' (decision 0202 calls it listenerToPut) */
export const toAnyFrame = (r: LatencyRecord): number | null => { const v = Object.values(r.frames); return v.length ? Math.min(...v) - r.tPage : null; };

/** the answer's frame that was followed by an animation frame first (the earliest such one) */
function firstAnswer(r: LatencyRecord): SeenFrame | null {
  let best: SeenFrame | null = null;
  for (const f of Object.values(r.answers)) if (f.raf !== null && f.raf >= 0 && (best === null || f.raf < (best.raf as number))) best = f;
  return best;
}

/** decision 0202: the event's own timeStamp -> the start of the first requestAnimationFrame callback after the answer's frame was drawn (null: no usable timeStamp, no answer yet, or no animation frame soon enough) */
export const toEventRaf = (r: LatencyRecord): number | null => { const f = firstAnswer(r); return f === null || r.tEvent === null ? null : (f.raf as number) - r.tEvent; };

/** decision 0205: a 'late' input's first later frame (within LATE_MS after the window), event.timeStamp -> the next animation frame after it was drawn. NOT an answer and not in the P2 number: it is shown beside it, because it may be a delayed repaint for this input */
export const toLateRaf = (r: LatencyRecord): number | null => { const f = r.outcome === 'late' ? r.lateFrame : null; return f === null || f.raf === null || f.raf < 0 || r.tEvent === null ? null : f.raf - r.tEvent; };

/**
 * The stages that add up to eventToRaf (ms). `toWorker` (page -> the engine took the event; includes the Worker's queue) and `fromWorker` (the Worker posted the frame -> the page's handler) cross from one thread's clock to the other's
 * (each is performance.timeOrigin + performance.now() there), so they can be a little off, even negative; the others are on one clock.
 */
export interface Stages {
  /** event.timeStamp -> the page's listener started: the browser's wait before it delivers the event */
  s0: number;
  /** listener -> the page made the input record (just before the postMessage) */
  s0b: number;
  toWorker: number;
  /** the engine took the event -> the Worker posted the answer's frame (the guest's processing, and the copy of the frame) */
  worker: number;
  fromWorker: number;
  /** the page's handler -> the putImageData calls returned */
  put: number;
  /** putImageData -> the start of the next requestAnimationFrame callback */
  raf: number;
}
export const STAGE_KEYS = ['s0', 's0b', 'toWorker', 'worker', 'fromWorker', 'put', 'raf'] as const;

export function stagesOf(r: LatencyRecord): Stages | null {
  const f = firstAnswer(r);
  if (f === null || r.tEvent === null || r.tListener === null || r.tInject === null) return null;
  return { s0: r.tListener - r.tEvent, s0b: r.tPage - r.tListener, toWorker: r.tInject - r.tPage, worker: f.postedAt - r.tInject, fromWorker: f.receivedAt - f.postedAt, put: f.at - f.receivedAt, raf: (f.raf as number) - f.at };
}
