'use strict';
// AudioWorkletProcessor that reads interleaved stereo float32 frames from a SharedArrayBuffer ring written by a Worker (the "engine").
// Header (Int32): 0 read index (frames, monotonic), 1 write index, 2 underrun blocks, 3 blocks processed, 4 wake word for the producer,
// 5 min fill seen (frames, after start), 6 max fill seen, 7 underrun frames (missing, after start), 8 started flag (set once the ring has been
// filled to half), 9 underrun blocks after start (2 counts all, including the start-up blocks before the producer has filled the ring). Data (Float32) starts at byte 64.
class RingReader extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const o = options.processorOptions;
    this.i = new Int32Array(o.sab);
    this.f = new Float32Array(o.sab, 64);
    this.cap = o.cap;
    this.started = 0;
    this.i[5] = 0x7fffffff;
  }
  process(inputs, outputs) {
    const out = outputs[0];
    const L = out[0];
    const R = out[1] || out[0];
    const n = L.length;                 // 128
    const i = this.i;
    const rd = Atomics.load(i, 0);
    const wr = Atomics.load(i, 1);
    const fill = wr - rd;
    if (!this.started && fill >= this.cap / 2) { this.started = 1; Atomics.store(i, 8, 1); }
    if (this.started) { if (fill < i[5]) i[5] = fill; if (fill > i[6]) i[6] = fill; }
    if (fill >= n) {
      const f = this.f;
      const cap = this.cap;
      for (let k = 0; k < n; k++) {
        const p = ((rd + k) % cap) * 2;
        L[k] = f[p];
        R[k] = f[p + 1];
      }
      Atomics.store(i, 0, rd + n);
    } else {
      Atomics.add(i, 2, 1);
      if (this.started) { Atomics.add(i, 9, 1); Atomics.add(i, 7, n - Math.max(0, fill)); }
    }
    Atomics.add(i, 3, 1);
    Atomics.add(i, 4, 1);
    Atomics.notify(i, 4, 1);
    return true;
  }
}
registerProcessor('ring-reader', RingReader);
