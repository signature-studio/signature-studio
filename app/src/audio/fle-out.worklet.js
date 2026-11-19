// AudioWorkletProcessor 'fle-out' (decision 0139 prototype, decision 0175 counting). Plain JavaScript: it is loaded by audioWorklet.addModule() and by the unit test, which runs process() with a scripted ring.
// While STATE == PLAYING (2) it reads 128 frames per quantum and advances RD by 128 whether or not the engine has published them: a frame the engine had not published is played as silence and counted (the output is a
// timeline, like a sound card's). While STATE != PLAYING it outputs silence and RD does not move.
// What is counted per quantum (the definition is in docs/0175-g7-audio-retry.md, S-ur-*):
//   underrun quantum   a quantum of the playing state in which fewer than 128 frames had been published (have < n); UR_FRAMES counts the missing frames
//   episode            a maximal run of consecutive underrun quanta; UR_MAXRUN is the longest run
//   warm-up            the underrun quanta before the first complete quantum of the playing state (the first block is written `lead` ahead only after the state became PLAYING): UR_STARTUP; the "steady" counters start after it
//   non-silent         a quantum of the playing state whose output has at least one non-zero sample
//   fill               frames published but not yet read at the start of a quantum (WR - RD, signed); the smallest one since the last post is posted as fmin: the margin the engine had (a quantum is missed when fill < 128)
// capture           (decision 0211: { cmd: 'capture', frames: N, timeline: true } also copies the silence of the missing frames, so that the copy is the output timeline and click intervals can be measured through under-runs) on request ({ cmd: 'capture', frames: N } on the port) the worklet copies the N frames that start at the first non-zero output sample and posts them once as { capture: Float32Array (L R interleaved), valid, firstFrame, validFrames }; valid is false when any frame of that stretch was missing (an underrun in it) and validFrames is then the number of frames before the first missing one (the frames after it are not aligned any more)
// Every 50 ms of audio time it posts its own view to the port: { frame, time, rd, wr, state, peak, sumSq, n, nonSilent, fmin } where peak, sumSq and n cover the output since the last post.
const W = { RD: 0, WR: 1, STATE: 2, Q_TOTAL: 8, Q_PLAYING: 9, UR_QUANTA: 10, UR_FRAMES: 11, FILL_MIN: 12, FILL_MAX: 13, UR_RUNS: 17, UR_MAXRUN: 18, UR_CURRUN: 19, NONSILENT_Q: 20, UR_STARTUP: 21, WARM: 22, UR_FIRST_AFTER_WARM: 23, UR_STEADY: 24, UR_STEADY_RUNS: 25, UR_STEADY_MAXRUN: 26 };

class FleOut extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const o = options.processorOptions;
    this.h = new Int32Array(o.sab, 0, 32);
    this.d = new Float32Array(o.sab, o.dataOffset, o.cap * 2);
    this.cap = o.cap;
    this.mask = o.cap - 1;
    this.nextPost = 0;
    this.peak = 0;
    this.sumSq = 0;
    this.n = 0;
    this.nonSilent = 0;
    this.steadyRun = 0;
    this.fmin = 0x7fffffff;
    this.cp = null;
    this.port.onmessage = (ev) => { const m = ev.data; if (m && m.cmd === 'capture' && m.frames > 0) this.cp = { n: m.frames, buf: new Float32Array(m.frames * 2), at: 0, started: false, valid: true, first: 0, missingAt: -1, timeline: m.timeline === true }; };
  }
  process(inputs, outputs) {
    const h = this.h, out = outputs[0], L = out[0], R = out[1] || out[0], n = L.length;
    const state = Atomics.load(h, W.STATE);
    Atomics.add(h, W.Q_TOTAL, 1);
    if (state === 2) {
      const rd = Atomics.load(h, W.RD) >>> 0, wr = Atomics.load(h, W.WR) >>> 0;
      const pos = rd;   // the ring position that is heard now
      let fill = (wr - pos) | 0;   // signed: negative when the engine is behind the output timeline
      const qp = Atomics.add(h, W.Q_PLAYING, 1) + 1;
      if (fill > this.cap) fill = this.cap;
      if (fill < this.fmin) this.fmin = fill;
      if (fill < Atomics.load(h, W.FILL_MIN)) Atomics.store(h, W.FILL_MIN, fill);
      if (fill > Atomics.load(h, W.FILL_MAX)) Atomics.store(h, W.FILL_MAX, fill);
      const have = fill >= n ? n : (fill > 0 ? fill : 0);
      const d = this.d, mask = this.mask;
      let anyNonZero = false, pk = this.peak, ss = this.sumSq;
      for (let k = 0; k < have; k++) {
        const p = ((pos + k) & mask) * 2;
        const l = d[p], r = d[p + 1];
        L[k] = l; R[k] = r;
        const a = l < 0 ? -l : l, b = r < 0 ? -r : r;
        if (a !== 0 || b !== 0) anyNonZero = true;
        if (a > pk) pk = a;
        if (b > pk) pk = b;
        ss += l * l + r * r;
        const c = this.cp;
        if (c !== null && c.at < c.n) {
          if (!c.started && (a !== 0 || b !== 0)) { c.started = true; c.first = pos + k; }
          if (c.started) { c.buf[2 * c.at] = l; c.buf[2 * c.at + 1] = r; c.at++; }
        }
      }
      if (this.cp !== null && this.cp.started && this.cp.at < this.cp.n && have < n) { if (this.cp.valid) this.cp.missingAt = this.cp.at; this.cp.valid = false; }
      for (let k = have; k < n; k++) { L[k] = 0; R[k] = 0; }
      if (this.cp !== null && this.cp.timeline && this.cp.started) for (let k = have; k < n && this.cp.at < this.cp.n; k++) this.cp.at++;   // the silence of a missing frame is part of the timeline (the buffer is zero already)
      this.peak = pk; this.sumSq = ss; this.n += n;
      if (anyNonZero) { Atomics.add(h, W.NONSILENT_Q, 1); this.nonSilent++; }
      const warm = Atomics.load(h, W.WARM) === 1;
      if (have < n) {
        Atomics.add(h, W.UR_QUANTA, 1); Atomics.add(h, W.UR_FRAMES, n - have);
        const run = Atomics.add(h, W.UR_CURRUN, 1) + 1;
        if (run === 1) Atomics.add(h, W.UR_RUNS, 1);
        if (run > Atomics.load(h, W.UR_MAXRUN)) Atomics.store(h, W.UR_MAXRUN, run);
        if (!warm) Atomics.add(h, W.UR_STARTUP, 1);
        else {
          Atomics.add(h, W.UR_STEADY, 1);
          this.steadyRun++;
          if (this.steadyRun === 1) Atomics.add(h, W.UR_STEADY_RUNS, 1);
          if (this.steadyRun > Atomics.load(h, W.UR_STEADY_MAXRUN)) Atomics.store(h, W.UR_STEADY_MAXRUN, this.steadyRun);
          if (Atomics.load(h, W.UR_FIRST_AFTER_WARM) === 0) Atomics.store(h, W.UR_FIRST_AFTER_WARM, qp);
        }
      } else {
        Atomics.store(h, W.UR_CURRUN, 0);
        this.steadyRun = 0;
        if (!warm) Atomics.store(h, W.WARM, 1);
      }
      Atomics.store(h, W.RD, (rd + n) >>> 0 | 0);
    } else {
      for (let k = 0; k < n; k++) { L[k] = 0; R[k] = 0; }
    }
    if (this.cp !== null && this.cp.at >= this.cp.n) { this.port.postMessage({ capture: this.cp.buf, valid: this.cp.valid, firstFrame: this.cp.first, validFrames: this.cp.valid ? this.cp.n : this.cp.missingAt }); this.cp = null; }
    if (currentFrame >= this.nextPost) {
      this.nextPost = currentFrame + sampleRate / 20;
      this.port.postMessage({ frame: currentFrame, time: currentTime, rd: Atomics.load(h, W.RD) >>> 0, wr: Atomics.load(h, W.WR) >>> 0, state: Atomics.load(h, W.STATE), peak: this.peak, sumSq: this.sumSq, n: this.n, nonSilent: this.nonSilent, fmin: this.fmin === 0x7fffffff ? null : this.fmin });
      this.fmin = 0x7fffffff;
      this.peak = 0; this.sumSq = 0; this.n = 0; this.nonSilent = 0;
    }
    return true;
  }
}
registerProcessor('fle-out', FleOut);
