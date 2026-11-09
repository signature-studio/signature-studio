// What the page measures about its own speed (decision 0152; targets P1-P3 of docs/milestones/performance-targets.md). Pure functions and a small recorder: no DOM, no network.
// Definitions (all times in ms on one clock, performance.timeOrigin + performance.now()):
//   update          a frame of a window surface was drawn on a canvas of the page (display method B: the page draws; with method A the Worker draws on an OffscreenCanvas and the page cannot see it)
//   interval        the time between two consecutive updates inside the window (the last 10 s)
//   median / p95    nearest rank over the sorted values: the value at index ceil(p/100 * n) - 1 (so the median of an even count is the lower middle one)
//   per second      updates in the window / the length of the window, where the window is min(10 s, now - observedSince)
//   input -> update (decision 0202, the P2 number) the time from the DOM event's own timeStamp (event.timeStamp: when the browser made the event) to the start of the first requestAnimationFrame callback after the first frame of any window that
//                   the Worker posted at or after the engine took the event was drawn (ui/latency.ts toEventRaf); the LAST 20 such events. It is the nearest this page gets to what a person feels: it contains the browser's wait before it delivers
//                   the event and its wait for the next animation frame after the draw. It stops at the animation frame: the compositor and the display come after it and are not measured.
//                   Decision 0205: that frame is the first one posted between the engine taking the event and the engine's idle that ends its processing (the frame that shows what the guest drew for the event); an input with no such frame
//                   has no value here (it is not in n, p50 or p95) and is counted in `late` / `noAnswer` instead. A press, or a move with a button down, may be answered by a later repaint (within 500 ms): it counts as `answered`.
//                   Decision 0205: an input whose visible change came after the engine's first idle ('late') is counted apart as lateVisible and, with the answered ones, as eventToRafWithLate (the first candidate for the P2 verdict).
//   listener -> put  version 2's definition, kept next to it under its own name (ui/latency.ts toAnyFrame): from the page's listener (tPage) to the putImageData of the same frame. 10-25 ms smaller than the one above (decision 0194).
import { percentile, STAGE_KEYS, toAnyFrame, toEventRaf, toLateRaf, stagesOf, type LatencyRecord, type Stages } from '../ui/latency.ts';

/** the numbers the page compares its measurements with (docs/milestones/performance-targets.md) */
export const TARGETS = { startupSeconds: 10, updatesPerSecond: 120, inputToUpdateP95Ms: 17 } as const;
/**
 * The upper limit of the engine's wasm memory (decision 0170): the memory section of the engine's wasm says initial 640 MB, maximum 2,048 MB (read from fl-engine-b1-D's wasm; build_nmod_wasm.sh sets no MAXIMUM_MEMORY, so it is Emscripten's
 * default). A page cannot read the maximum of a WebAssembly.Memory, so it is a constant here: a build with another maximum needs another number. The engine's memory only grows; when it cannot grow the engine's allocations fail.
 */
export const WASM_MEMORY_LIMIT_BYTES = 2048 * 1024 * 1024;
export const RECENT_WINDOW_MS = 10_000;
export const LATENCY_LAST = 20;

export interface IntervalStats { n: number; intervalMedianMs: number | null; intervalP95Ms: number | null; perSecond: number | null }

/** `times` ascending; `now` and `observedSince` on the same clock (observedSince null: unknown, no rate) */
export function intervalStats(times: readonly number[], now: number, observedSince: number | null, windowMs = RECENT_WINDOW_MS): IntervalStats {
  const inWin = times.filter((t) => t > now - windowMs && t <= now);
  const gaps: number[] = [];
  for (let i = 1; i < inWin.length; i++) gaps.push((inWin[i] as number) - (inWin[i - 1] as number));
  gaps.sort((a, b) => a - b);
  const span = observedSince === null ? null : Math.min(windowMs, now - observedSince);
  return {
    n: inWin.length,
    intervalMedianMs: percentile(gaps, 50),
    intervalP95Ms: percentile(gaps, 95),
    perSecond: span === null || span <= 0 ? null : inWin.length / (span / 1000),
  };
}

/** n of the last events that have the measure, the quantiles over them (nearest rank), and those values, oldest first (at most LATENCY_LAST) */
export interface LatencyStat { n: number; p50Ms: number | null; p95Ms: number | null; maxMs: number | null; lastMs: number[] }
/** the median and the 95th percentile of each stage (ms) over the events of eventToRaf; the medians do not add up to the median of the sum */
export interface StageStats { n: number; p50: Stages; p95: Stages }

export interface InputLatencyStats {
  /** events the page sent / the engine processed / that were dropped or merged / still waiting */
  sent: number; processed: number; dropped: number; pending: number;
  /** decision 0205: of the processed events, those with an answer frame (in the processing window, or a held input's later repaint) / with no answer but a frame within 150 ms after / with no frame soon enough (a button released, a move over a
   *  place that does not react: these are the events that changed no pixel). processed = answered + late + noAnswer + (those still within their wait). */
  answered: number; late: number; noAnswer: number;
  /** decision 0202, the P2 number: event.timeStamp -> the next animation frame after the answer was drawn */
  eventToRaf: LatencyStat;
  /** version 2's number, kept: the page's listener -> the answer's putImageData */
  listenerToPut: LatencyStat;
  /**
   * decision 0205, only when every frame the page receives changed pixels (?dedupe=1; null otherwise: with dedupe off the page cannot tell a changed frame from a redraw): the 'late' inputs (no frame in the processing window, a first frame within
   * 150 ms after it, so a visible change that came after the engine's first idle): event.timeStamp -> the next animation frame after that frame. Not an answer, so not in eventToRaf; the frame may be a delayed repaint for the input or an unrelated change
   */
  lateVisible: LatencyStat | null;
  /** decision 0205, as above (null when dedupe is not on): eventToRaf and lateVisible together, in the order of the inputs - the first candidate for the P2 verdict (the PM decides); eventToRaf alone leaves out the visible changes that came late */
  eventToRafWithLate: LatencyStat | null;
  /** where the time of eventToRaf goes; null: no event has it */
  stages: StageStats | null;
}

const round3 = (x: number): number => Math.round(x * 1000) / 1000;

function latencyStat(values: readonly number[], last: number): LatencyStat {
  const lastMs = values.slice(-last).map(round3);
  const sorted = [...lastMs].sort((a, b) => a - b);
  return { n: lastMs.length, p50Ms: percentile(sorted, 50), p95Ms: percentile(sorted, 95), maxMs: sorted.length ? (sorted[sorted.length - 1] as number) : null, lastMs };
}

export function inputLatencyStats(records: readonly LatencyRecord[], totals: { sent: number; processed: number; dropped: number; pending: number; answered: number; late: number; noAnswer: number }, last = LATENCY_LAST, framesAreChanges = false): InputLatencyStats {
  const old: number[] = [];
  const fresh: number[] = [];
  const lateV: number[] = [];
  const both: number[] = [];
  const staged: (Stages | null)[] = [];
  for (const r of records) {
    if (r.dropped !== null) continue;
    const v = toAnyFrame(r);
    if (v !== null) old.push(v);
    const e = toEventRaf(r);
    if (e !== null) { fresh.push(e); staged.push(stagesOf(r)); both.push(e); }
    const l = toLateRaf(r);
    if (l !== null && framesAreChanges) { lateV.push(l); both.push(l); }
  }
  const eventToRaf = latencyStat(fresh, last);
  const used = staged.slice(-eventToRaf.n).filter((x): x is Stages => x !== null);
  const q = (p: number): Stages => {
    const o = {} as Record<string, number>;
    for (const k of STAGE_KEYS) o[k] = round3(percentile(used.map((x) => x[k]).sort((a, b) => a - b), p) as number);
    return o as unknown as Stages;
  };
  return { ...totals, eventToRaf, listenerToPut: latencyStat(old, last), lateVisible: framesAreChanges ? latencyStat(lateV, last) : null, eventToRafWithLate: framesAreChanges ? latencyStat(both, last) : null, stages: used.length ? { n: used.length, p50: q(50), p95: q(95) } : null };
}

/** keeps the instants of the updates and of the engine's idles (bounded), and since when the rate is observed */
export class PerfRecorder {
  private updates: number[] = [];
  private idles: number[] = [];
  private updateTotal = 0;
  private idleTotal = 0;
  private since: number | null = null;
  private readonly cap: number;
  constructor(cap = 4096) { this.cap = cap; }

  /** the engine starts at `at`: the rates are counted from here */
  start(at: number): void { this.since = at; }
  update(at: number): void { this.updateTotal++; this.updates.push(at); if (this.since === null) this.since = at; if (this.updates.length > this.cap * 2) this.updates.splice(0, this.updates.length - this.cap); }
  /** an idle report at `at`; `count` idles happened since the previous report (decision 0170: the Worker reports at most every 40 ms, the total stays exact, the times are those of the reports) */
  idle(at: number, count = 1): void { this.idleTotal += count; this.idles.push(at); if (this.idles.length > this.cap * 2) this.idles.splice(0, this.idles.length - this.cap); }

  snapshot(now: number): { updates: { total: number; recent: IntervalStats }; idles: { total: number; recent: IntervalStats }; observedSince: number | null } {
    return {
      updates: { total: this.updateTotal, recent: intervalStats(this.updates, now, this.since) },
      idles: { total: this.idleTotal, recent: intervalStats(this.idles, now, this.since) },
      observedSince: this.since,
    };
  }
}
