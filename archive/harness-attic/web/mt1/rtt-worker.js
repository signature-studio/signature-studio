// mt1: round-trip latency of a SharedArrayBuffer "process request" between a producer (page main thread or a Worker) and K plugin Workers (decision 0173).
// Per plugin slot (one SAB): Int32 ctrl[0] = request sequence (producer -> plugin), ctrl[1] = response sequence (plugin -> producer), ctrl[2] = work in microseconds
// (the plugin busy-waits that long between request and response, standing for the DSP), bytes 64.. = input block (FRAMES x 2 ch x float32), then the output block.
const FRAMES = 4096, BLK = FRAMES * 2 * 4;
export const OFF_IN = 64, OFF_OUT = 64 + BLK, SLOT_BYTES = 64 + 2 * BLK;
const now = () => performance.now();

function pluginLoop(sab, mode) {          // a plugin Worker: wait for a request, copy input to output (a stand-in for process), answer
  const ctrl = new Int32Array(sab, 0, 16);
  const inp = new Float32Array(sab, OFF_IN, BLK / 4), out = new Float32Array(sab, OFF_OUT, BLK / 4);
  let seen = 0;
  for (;;) {
    if (mode === 'spin') { while (Atomics.load(ctrl, 0) === seen) { /* spin */ } }
    else { while (Atomics.load(ctrl, 0) === seen) Atomics.wait(ctrl, 0, seen); }
    seen = Atomics.load(ctrl, 0);
    if (seen < 0) return;
    out.set(inp);
    const w = Atomics.load(ctrl, 2);
    if (w > 0) { const t = now() + w / 1000; while (now() < t) { /* DSP */ } }
    Atomics.store(ctrl, 1, seen);
    Atomics.notify(ctrl, 1);
  }
}

function producerLoop(sabs, rounds, workUs, mode, warm) {   // fan-out to every plugin, then wait for all
  const K = sabs.length;
  const ctrls = sabs.map((s) => new Int32Array(s, 0, 16));
  const ins = sabs.map((s) => new Float32Array(s, OFF_IN, BLK / 4)), outs = sabs.map((s) => new Float32Array(s, OFF_OUT, BLK / 4));
  const src = new Float32Array(BLK / 4); for (let i = 0; i < src.length; i++) src[i] = Math.sin(i * 0.01);
  const dst = new Float32Array(BLK / 4);
  for (const c of ctrls) Atomics.store(c, 2, Math.round(workUs));
  const lat = new Float64Array(rounds);
  for (let r = 1; r <= rounds + warm; r++) {
    const t0 = now();
    for (let k = 0; k < K; k++) { ins[k].set(src); Atomics.store(ctrls[k], 0, r); Atomics.notify(ctrls[k], 0); }
    for (let k = 0; k < K; k++) {
      const c = ctrls[k];
      if (mode === 'spin') { while (Atomics.load(c, 1) !== r) { /* spin */ } }
      else { for (;;) { const v = Atomics.load(c, 1); if (v === r) break; Atomics.wait(c, 1, v); } }
      dst.set(outs[k]);
    }
    const t1 = now();
    if (r > warm) lat[r - warm - 1] = (t1 - t0) * 1000;
  }
  for (const c of ctrls) { Atomics.store(c, 0, -1); Atomics.notify(c, 0); }
  return lat;
}

self.onmessage = (e) => {
  const m = e.data;
  if (m.role === 'plugin') { postMessage({ type: 'plugin-start' }); pluginLoop(m.sab, m.mode); postMessage({ type: 'plugin-exit' }); }
  else if (m.role === 'producer') {
    const lat = producerLoop(m.sabs, m.rounds, m.workUs, m.mode, m.warm);
    postMessage({ type: 'lat', lat }, [lat.buffer]);
  }
};
