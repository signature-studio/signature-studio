// How the page presents the worklet's counters (decision 0175). The counting itself is done per quantum by fle-out.worklet.js (unit-tested by running that file); this class turns the counters into the numbers of
// P3 ("Song Mode playback without dropouts"): the strict count of underrun quanta (everything after the playing state began), the steady count (after warm-up), the episodes and the longest run, and the duration they
// stand for. A quantum is 128 frames; at the ring's rate it is 128 / rate seconds.
import { QUANTUM, type RingCounters } from './ring-layout.ts';

export interface OutputLevel { peak: number; rms: number }

export class UnderrunStats {
  readonly c: RingCounters;
  readonly level: OutputLevel;
  constructor(c: RingCounters, level: OutputLevel) { this.c = c; this.level = level; }

  /** seconds of output (quanta of the playing state x 128 / rate) */
  get playingSeconds(): number { return this.c.rate > 0 ? (this.c.playing * QUANTUM) / this.c.rate : 0; }
  /** underrun quanta, warm-up included */
  get total(): number { return this.c.urQuanta; }
  /** underrun quanta after warm-up (the first complete quantum of the playing state) */
  get steady(): number { return this.c.urSteady; }
  /** the number P3 shows: 0 only when not a single quantum of the playing state lacked frames after warm-up */
  get p3(): number { return this.c.urSteady; }
  /** seconds of output that were silence only because the engine had not delivered (after warm-up) */
  get steadyMissingSeconds(): number { return this.c.rate > 0 ? (this.c.urSteady * QUANTUM) / this.c.rate : 0; }
  get silentQuanta(): number { return this.c.playing - this.c.nonSilent; }
  /** the share of the playing quanta that were underruns (0..1) */
  get share(): number { return this.c.playing > 0 ? this.c.urQuanta / this.c.playing : 0; }
}
