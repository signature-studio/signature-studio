// mt1: the "engine" side of the pipeline prototype: a Worker that sends 4096-frame (32 KiB) blocks to K plugin Workers over SAB slots and waits for the answers, either in parallel (fan-out)
// or as a chain (each plugin's output is the next plugin's input), optionally paced at the real-time block period (92.88 ms). It stands for the engine's mixer thread calling process().
const FRAMES = 4096, BLK = FRAMES * 2 * 4, OFF_IN = 64, OFF_OUT = 64 + BLK;
self.onmessage = (e) => {
  const { sabs, blocks, topo, paced, periodMs } = e.data;
  const K = sabs.length;
  const ctrls = sabs.map((s) => new Int32Array(s, 0, 16));
  const ins = sabs.map((s) => new Float32Array(s, OFF_IN, BLK / 4)), outs = sabs.map((s) => new Float32Array(s, OFF_OUT, BLK / 4));
  const src = new Float32Array(BLK / 4); for (let i = 0; i < src.length; i++) src[i] = Math.sin(i * 0.01);
  const dst = new Float32Array(BLK / 4);
  const sleepSab = new Int32Array(new SharedArrayBuffer(4));
  const waitResp = (k, r) => { const c = ctrls[k]; for (;;) { const v = Atomics.load(c, 1); if (v === r) return; Atomics.wait(c, 1, v); } };
  const lat = new Float64Array(blocks), late = new Float64Array(blocks);
  const T0 = performance.now() + 20;
  for (let b = 1; b <= blocks; b++) {
    const sched = T0 + (b - 1) * periodMs;
    if (paced) { const d = sched - performance.now(); if (d > 0) Atomics.wait(sleepSab, 0, 0, d); }
    const t0 = performance.now();
    if (topo === 'par') {
      for (let k = 0; k < K; k++) { ins[k].set(src); Atomics.store(ctrls[k], 0, b); Atomics.notify(ctrls[k], 0); }
      for (let k = 0; k < K; k++) { waitResp(k, b); dst.set(outs[k]); }
    } else {
      ins[0].set(src);
      for (let k = 0; k < K; k++) {
        if (k > 0) ins[k].set(outs[k - 1]);
        Atomics.store(ctrls[k], 0, b); Atomics.notify(ctrls[k], 0); waitResp(k, b);
      }
      dst.set(outs[K - 1]);
    }
    const t1 = performance.now();
    lat[b - 1] = t1 - t0; late[b - 1] = t0 - sched;
  }
  postMessage({ type: 'lat', lat, late }, [lat.buffer, late.buffer]);
};
