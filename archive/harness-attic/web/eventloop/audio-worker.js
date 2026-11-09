'use strict';
// The "engine side" of the audio path: a Worker in a blocking loop (Atomics.wait) that renders chunks of audio into the ring when there is room,
// optionally stalls (CPU spin) every c.stallEveryMs for c.stallMs (the engine busy with something else), and stops at c.ms.
importScripts('ring.js');
onmessage = (e) => {
  const c = e.data;
  const i = new Int32Array(c.sab);
  const f = new Float32Array(c.sab, 64);
  const cap = c.cap;
  const chunk = c.chunk;
  let phase = 0;
  const w = 2 * Math.PI * 440 / c.sampleRate;
  let produced = 0;
  let waits = 0;
  let stalls = 0;
  const t0 = performance.now();
  let nextStall = t0 + (c.stallEveryMs || 1e9);
  postMessage({ type: 'ready' });
  while (performance.now() - t0 < c.ms) {
    const rd = Atomics.load(i, 0);
    const wr = Atomics.load(i, 1);
    if (cap - (wr - rd) >= chunk) {
      if (c.stallMs && performance.now() >= nextStall) { spin(c.stallMs); stalls++; nextStall = performance.now() + c.stallEveryMs; }
      if (c.loadMs) spin(c.loadMs);   // render cost of one chunk
      for (let k = 0; k < chunk; k++) {
        const p = ((wr + k) % cap) * 2;
        const s = Math.sin(phase) * 0.1;
        f[p] = s; f[p + 1] = s;
        phase += w;
      }
      Atomics.store(i, 1, wr + chunk);
      produced += chunk;
    } else {
      waits++;
      const cur = Atomics.load(i, 4);
      Atomics.wait(i, 4, cur, 4);       // woken by the worklet after every block, or after 4 ms
    }
  }
  postMessage({ type: 'result', produced, waits, stalls, elapsed_ms: performance.now() - t0 });
};
