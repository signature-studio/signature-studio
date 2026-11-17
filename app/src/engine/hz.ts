// The refresh rate of the virtual display the guest is paced by (decision 0182; the runtime side is decision 0166: lift/runtime/fle_thread.c reads FLE_DWM_HZ and FLE_DISPLAYSETTINGS_FREQUENCY from the engine's
// environment, 24..1000, default 60). The page measures the rate of requestAnimationFrame for `windowMs` and rounds it to the nearest standard rate (STANDARD_HZ, nearest in the ratio sense; the highest standard rate is 120, so a faster screen gets 120: decision 0268); if requestAnimationFrame
// does not deliver enough frames (a hidden tab, a browser that stops it) the answer is the default and the page says so. It passes the SAME value for both variables.
// Everything here is pure: the frame clock and the timer are passed in (test/unit/hz.test.ts), the page wires them to requestAnimationFrame and setTimeout (main.ts).

export const HZ_MIN = 24;
export const HZ_MAX = 1000;
/** S-hz-window: the stretch of frames that is measured, ms. 500 ms is 30 frames at 60 Hz and 120 at 240 Hz, which the median needs for the rounding to the standard rates (decision 0243: it was 1000 ms; the measurement runs while the assets download, and the part of it that is still running when they are done is added to the start-up) */
export const HZ_WINDOW_MS = 500;
/** the engine's default (fle_thread.c FLE_DWM_HZ, fle_monitor.h FLE_DISPLAYSETTINGS_FREQUENCY): what the engine runs at until the screen's rate is measured */
export const HZ_DEFAULT = 60;
/** S-hz-standard: the refresh rates displays are sold with that the measured rate is rounded to (no source for the list; 50 and 100 Hz panels round to 60 and 90). Decision 0268: the list ends at 120, so 144, 165, 240 Hz and faster screens get 120. Why: the guest's drawing tick follows this rate, and above 120 it adds work (about 3 ms of the engine Worker per tick at idle) without a picture the display can use, and the work comes out of the mixer's share (decision 0263, measured: write rate 0.326 at 120 Hz, 0.310 at 144, 0.291 at 240). To be looked at again when the mixer keeps up (P3). */
export const STANDARD_HZ: readonly number[] = [60, 75, 90, 120];

/** S-hz-round: the nearest standard rate by ratio (|ln(hz / s)|); at or below the lowest standard (60) and at or above the highest (120) the end value */
export function roundToStandardHz(measured: number): number {
  if (!(measured > 0) || !Number.isFinite(measured)) return HZ_DEFAULT;
  let best = STANDARD_HZ[0] as number, bestD = Infinity;
  for (const s of STANDARD_HZ) {
    const d = Math.abs(Math.log(measured / s));
    if (d < bestD) { bestD = d; best = s; }
  }
  return best;
}

/** the engine environment for a rate: both variables, the same value */
export function hzEnv(hz: number): Record<string, string> {
  return { FLE_DWM_HZ: String(hz), FLE_DISPLAYSETTINGS_FREQUENCY: String(hz) };
}

export interface FrameHost {
  /** calls `cb` with the frame's timestamp (ms, DOMHighResTimeStamp) at the next frame */
  frame(cb: (t: number) => void): void;
  setTimer(fn: () => void, ms: number): unknown;
  clearTimer(h: unknown): void;
}
export interface MeasureOptions {
  /** S-hz-skip: frame intervals ignored at the start (the first frames of a page are late: layout, style, the first paint) */
  skipIntervals?: number;
  /** S-hz-window: the measured stretch after the skipped intervals, ms */
  windowMs?: number;
  /** S-hz-min: fewest intervals that make an answer */
  minIntervals?: number;
  /** S-hz-giveup: after this many ms without enough intervals the answer is the default */
  giveUpMs?: number;
}
export interface HzMeasurement {
  /** the median frame interval's rate, Hz; null when there were not enough frames */
  measuredHz: number | null;
  intervals: number;
  hz: number;
  /** why the answer is what it is, for the log */
  note: string;
}

/** Measures the page's frame rate: the median of the intervals between frame timestamps over `windowMs`, after `skipIntervals`. The median keeps a few late frames (the page's own start-up work) from moving the answer. */
export function measureFrameRate(host: FrameHost, opts: MeasureOptions = {}): Promise<HzMeasurement> {
  const skip = opts.skipIntervals ?? 5, windowMs = opts.windowMs ?? HZ_WINDOW_MS, minN = opts.minIntervals ?? 20, giveUp = opts.giveUpMs ?? 4000;
  return new Promise((resolve) => {
    let done = false;
    const stamps: number[] = [];
    const finish = (): void => {
      if (done) return;
      done = true;
      host.clearTimer(timer);
      const iv: number[] = [];
      for (let i = 1; i < stamps.length; i++) iv.push((stamps[i] as number) - (stamps[i - 1] as number));
      const used = iv.slice(skip);
      if (used.length < minN) {
        resolve({ measuredHz: null, intervals: used.length, hz: HZ_DEFAULT, note: `only ${used.length} frame intervals were seen (at least ${minN} are needed; a hidden tab or a browser that stops animation frames does this): ${HZ_DEFAULT} Hz` });
        return;
      }
      const sorted = used.slice().sort((a, b) => a - b);
      const mid = sorted.length >> 1;
      const median = sorted.length % 2 ? (sorted[mid] as number) : ((sorted[mid - 1] as number) + (sorted[mid] as number)) / 2;
      if (!(median > 0)) { resolve({ measuredHz: null, intervals: used.length, hz: HZ_DEFAULT, note: `the median frame interval was ${median} ms: ${HZ_DEFAULT} Hz` }); return; }
      const measuredHz = 1000 / median;
      const hz = roundToStandardHz(measuredHz);
      resolve({ measuredHz, intervals: used.length, hz, note: `${used.length} frame intervals, median ${median.toFixed(2)} ms = ${measuredHz.toFixed(1)} Hz, rounded to ${hz} Hz` });
    };
    const timer = host.setTimer(finish, giveUp);
    const tick = (t: number): void => {
      if (done) return;
      stamps.push(t);
      const first = stamps[skip];
      if (first !== undefined && t - first >= windowMs && stamps.length - 1 - skip >= minN) { finish(); return; }
      host.frame(tick);
    };
    host.frame(tick);
  });
}

export interface HzApplied {
  /** the rate put into the environment */
  hz: number;
  warning: string | null;
  /** lines for the page's log */
  lines: { level: 'info' | 'warn'; text: string }[];
}

/** Puts the measured rate into `env` (the manifest's engine.env, which the Worker hands to the engine): both variables, the same value. `measured` is the result of measureFrameRate (null: no measurement, the engine keeps 60 Hz). */
export function applyHz(measured: HzMeasurement | null, env: Record<string, string>): HzApplied {
  const lines: HzApplied['lines'] = [];
  const hz = measured ? measured.hz : HZ_DEFAULT;
  if (measured) lines.push({ level: measured.measuredHz === null ? 'warn' : 'info', text: `display refresh rate: ${measured.note}` });
  else lines.push({ level: 'warn', text: `display refresh rate: no measurement: ${HZ_DEFAULT} Hz` });
  Object.assign(env, hzEnv(hz));
  lines.push({ level: 'info', text: `display refresh rate ${hz} Hz for the engine (FLE_DWM_HZ=${hz}, FLE_DISPLAYSETTINGS_FREQUENCY=${hz})` });
  return { hz, warning: null, lines };
}
