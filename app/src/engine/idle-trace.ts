// A fingerprint of the time the guest lives through (decision 0170): every idle of the engine (its idle number, its virtual clock and its next deadline) is folded into a running FNV-1a hash, and the hash is
// kept at every `every`-th idle. The virtual clock of the engine does not depend on how the host paces its idles (only WHEN a deadline is delivered does), so two runs of the same engine without input must have
// the same checkpoints, whatever the pacing: that is how a browser run shows that a change of the pacing did not change what the guest sees. Pure.
const PRIME = 0x01000193;
const mix = (h: number, x: number): number => Math.imul(h ^ (x >>> 0), PRIME) >>> 0;
const lo = (v: number): number => v % 4294967296;
const hi = (v: number): number => Math.floor(v / 4294967296);

export interface TraceCheckpoint { n: number; hash: number }

export class IdleTrace {
  private h = 0x811c9dc5;
  private readonly every: number;
  constructor(every = 500) { this.every = every; }

  /** folds the idle `n` in; returns the checkpoint when `n` is a multiple of `every` */
  push(n: number, nowTicks: number, nextTicks: number): TraceCheckpoint | null {
    let h = this.h;
    h = mix(h, n);
    h = mix(h, lo(nowTicks)); h = mix(h, hi(nowTicks));
    h = mix(h, lo(nextTicks)); h = mix(h, hi(nextTicks));
    this.h = h;
    return n % this.every === 0 ? { n, hash: h } : null;
  }
}
