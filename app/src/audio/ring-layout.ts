// The layout of the SharedArrayBuffer ring between the engine's Worker (producer, written by the engine's own EM_JS in lift/runtime/fle_audio_ring.c) and the AudioWorklet (consumer), decision 0139 version 1, plus the words
// that this front end adds for its own counting (decision 0175). The engine never reads or writes words 17 and up. All counters are Int32 words read and written with Atomics; the frame counters are modulo 2^32.
// Float32 data follows at DATA_OFFSET: interleaved L R, `cap` frames.
export const W = {
  RD: 0, WR: 1, STATE: 2, RATE: 3, CAP: 4, MAGIC: 5, VERSION: 6,
  Q_TOTAL: 8, Q_PLAYING: 9, UR_QUANTA: 10, UR_FRAMES: 11, FILL_MIN: 12, FILL_MAX: 13, PUBLISHES: 14, PUBLISHED_FRAMES: 15, SKIPPED_FRAMES: 16,
  /** decision 0175: episodes (maximal runs of consecutive underrun quanta) */
  UR_RUNS: 17,
  /** the longest run, in quanta */
  UR_MAXRUN: 18,
  /** the length of the run that is going on (0: the last quantum was complete) */
  UR_CURRUN: 19,
  /** quanta (while playing) whose output held at least one non-zero sample */
  NONSILENT_Q: 20,
  /** underrun quanta before the first complete quantum of the playing state (warm-up) */
  UR_STARTUP: 21,
  /** 1 once a complete quantum has been played in the playing state */
  WARM: 22,
  /** the quantum number (Q_PLAYING) of the first underrun after warm-up, 0 when there was none */
  UR_FIRST_AFTER_WARM: 23,
  /** underrun quanta after warm-up */
  UR_STEADY: 24,
  /** episodes after warm-up */
  UR_STEADY_RUNS: 25,
  /** longest run after warm-up */
  UR_STEADY_MAXRUN: 26,
} as const;
export const MAGIC_VALUE = 0x52414c46;
export const VERSION_VALUE = 1;
export const WORDS = 32;
export const DATA_OFFSET = 256;
export const STATE_CLOSED = 0;
export const STATE_OPEN = 1;
export const STATE_PLAYING = 2;
/** one render quantum of the Web Audio API */
export const QUANTUM = 128;

export const ringBytes = (capFrames: number): number => DATA_OFFSET + capFrames * 2 * 4;

export function makeRingBuffer(capFrames: number, rate: number): SharedArrayBuffer {
  if (capFrames < 1024 || (capFrames & (capFrames - 1)) !== 0) throw new Error(`the ring capacity must be a power of two of at least 1024 frames (got ${capFrames})`);
  const sab = new SharedArrayBuffer(ringBytes(capFrames));
  const h = new Int32Array(sab, 0, WORDS);
  h[W.CAP] = capFrames;
  h[W.RATE] = rate;
  h[W.MAGIC] = MAGIC_VALUE;
  h[W.VERSION] = VERSION_VALUE;
  h[W.FILL_MIN] = 0x7fffffff;
  return sab;
}

export interface RingCounters {
  rd: number; wr: number; state: number; rate: number;
  quanta: number; playing: number;
  urQuanta: number; urFrames: number; urRuns: number; urMaxRun: number; urCurRun: number;
  nonSilent: number; urStartup: number; warm: boolean; urFirstAfterWarm: number; urSteady: number; urSteadyRuns: number; urSteadyMaxRun: number;
  fillMin: number; fillMax: number; publishes: number; publishedFrames: number; skippedFrames: number;
}

export function readCounters(h: Int32Array): RingCounters {
  const g = (i: number): number => Atomics.load(h, i);
  const fmin = g(W.FILL_MIN);
  return {
    rd: g(W.RD) >>> 0, wr: g(W.WR) >>> 0, state: g(W.STATE), rate: g(W.RATE),
    quanta: g(W.Q_TOTAL), playing: g(W.Q_PLAYING),
    urQuanta: g(W.UR_QUANTA), urFrames: g(W.UR_FRAMES), urRuns: g(W.UR_RUNS), urMaxRun: g(W.UR_MAXRUN), urCurRun: g(W.UR_CURRUN),
    nonSilent: g(W.NONSILENT_Q), urStartup: g(W.UR_STARTUP), warm: g(W.WARM) === 1, urFirstAfterWarm: g(W.UR_FIRST_AFTER_WARM), urSteady: g(W.UR_STEADY), urSteadyRuns: g(W.UR_STEADY_RUNS), urSteadyMaxRun: g(W.UR_STEADY_MAXRUN),
    fillMin: fmin === 0x7fffffff ? 0 : fmin, fillMax: g(W.FILL_MAX), publishes: g(W.PUBLISHES), publishedFrames: g(W.PUBLISHED_FRAMES), skippedFrames: g(W.SKIPPED_FRAMES),
  };
}
