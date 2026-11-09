// The input stream of the OTT harness (lift/harness/vst/main_vst.c) reproduced bit for bit: a 440 Hz / 660 Hz sine pair plus xorshift32 noise, float32, 44.1 kHz. Feeding it to the plugin Worker
// must give the harness' own out_hash (8104f50e938ccf36 for the default 40 blocks, decision 0156/0164). Used by the tests and by the paced driver; not part of the engine.
export class OttTestSignal {
  private rng = 0x12345678;
  private n0 = 0;
  readonly scale: number;
  readonly silence: boolean;
  readonly rate: number;

  constructor(opts: { scale?: number; silence?: boolean; rate?: number } = {}) {
    this.scale = opts.scale ?? 1;
    this.silence = opts.silence ?? false;
    this.rate = opts.rate ?? 44100;
  }

  private nz(): number {
    let r = this.rng;
    r = (r ^ (r << 13)) >>> 0;
    r = (r ^ (r >>> 17)) >>> 0;
    r = (r ^ (r << 5)) >>> 0;
    this.rng = r;
    return (r | 0) / 2147483648;
  }

  /** Fill the next block (planar L/R). */
  next(frames: number, l: Float32Array, r: Float32Array): void {
    for (let i = 0; i < frames; i++) {
      const t = (this.n0 + i) / this.rate;
      const a = this.nz();
      l[i] = this.silence ? 0 : this.scale * (0.5 * Math.sin(6.283185307179586 * 440.0 * t) + 0.05 * a);
      const b = this.nz();
      r[i] = this.silence ? 0 : this.scale * (0.4 * Math.sin(6.283185307179586 * 660.0 * t) + 0.05 * b);
    }
    this.n0 += frames;
  }
}
