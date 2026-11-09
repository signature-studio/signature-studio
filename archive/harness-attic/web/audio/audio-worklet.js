'use strict';
// AudioWorkletProcessor 'fle-out' (decision 0139, PROVISIONAL): plays the ring of ring.js. While STATE == PLAYING it reads 128 frames per quantum and advances RD by 128 whether or not the engine has published them: a frame the
// engine had not published is played as silence and counted (the output is a timeline, like a sound card's). While STATE != PLAYING it outputs silence and RD does not move. It posts a sample of its own counters to the port every
// 50 ms of audio time: { frame, time, rd, wr, state, q, qp, ur, urf, fmin, fmax }.
class FleOut extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const o = options.processorOptions;
    this.h = new Int32Array(o.sab, 0, 32);
    this.d = new Float32Array(o.sab, o.dataOffset, o.cap * 2);
    this.cap = o.cap;
    this.mask = o.cap - 1;
    this.nextPost = 0;
    this.warm = 0;
  }
  process(inputs, outputs) {
    const h = this.h, out = outputs[0], L = out[0], R = out[1] || out[0], n = L.length;
    const state = Atomics.load(h, 2);
    Atomics.add(h, 8, 1);
    if (state === 2) {
      const rd = Atomics.load(h, 0) >>> 0, wr = Atomics.load(h, 1) >>> 0;
      let fill = (wr - rd) | 0;   // signed: negative when the engine is behind the output timeline
      Atomics.add(h, 9, 1);
      if (fill > this.cap) fill = this.cap;
      if (fill < Atomics.load(h, 12)) Atomics.store(h, 12, fill);
      if (fill > Atomics.load(h, 13)) Atomics.store(h, 13, fill);
      const have = fill >= n ? n : (fill > 0 ? fill : 0);
      const d = this.d, mask = this.mask;
      for (let k = 0; k < have; k++) { const p = ((rd + k) & mask) * 2; L[k] = d[p]; R[k] = d[p + 1]; }
      for (let k = have; k < n; k++) { L[k] = 0; R[k] = 0; }
      if (have < n) { Atomics.add(h, 10, 1); Atomics.add(h, 11, n - have); }
      Atomics.store(h, 0, (rd + n) >>> 0 | 0);
    } else {
      for (let k = 0; k < n; k++) { L[k] = 0; R[k] = 0; }
    }
    if (currentFrame >= this.nextPost) {
      this.nextPost = currentFrame + sampleRate / 20;
      this.port.postMessage({ frame: currentFrame, time: currentTime, rd: Atomics.load(h, 0) >>> 0, wr: Atomics.load(h, 1) >>> 0, state: Atomics.load(h, 2), q: Atomics.load(h, 8), qp: Atomics.load(h, 9),
        ur: Atomics.load(h, 10), urf: Atomics.load(h, 11), fmin: Atomics.load(h, 12), fmax: Atomics.load(h, 13) });
    }
    return true;
  }
}
registerProcessor('fle-out', FleOut);
