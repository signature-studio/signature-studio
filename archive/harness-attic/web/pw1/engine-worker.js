// pw1 (decision 0186): the "engine" side of the real-time run. A Worker that, every period (4096 frames at 44.1 kHz = 92.879 ms) after T0, publishes one 32 KiB block to each of K plug-in
// Workers over their SAB slots (fan-out, all plug-ins of a block in parallel) and waits for all the answers before it sleeps for the next period. Like a mixer thread that needs every
// plug-in's output of block b before it can finish block b: when a block is late the next one starts at once (no skipping), so a late block makes the followers late too (backlog).
// Per block it records (epoch ms = performance.timeOrigin + performance.now()): the schedule time, the time it released the block, and when each plug-in's answer was seen.
const FRAMES = 4096, BLK = FRAMES * 2 * 4, OFF_IN = 64, OFF_OUT = 64 + BLK;
const abs = () => performance.timeOrigin + performance.now();
self.onmessage = (e) => {
  const { sabs, blocks, periodMs, t0 } = e.data;
  const K = sabs.length;
  const ctrls = sabs.map((s) => new Int32Array(s, 0, 16));
  const ins = sabs.map((s) => new Float32Array(s, OFF_IN, BLK / 4)), outs = sabs.map((s) => new Float32Array(s, OFF_OUT, BLK / 4));
  const src = new Float32Array(BLK / 4); for (let i = 0; i < src.length; i++) src[i] = Math.sin(i * 0.01);
  const dst = new Float32Array(BLK / 4);
  const sleepSab = new Int32Array(new SharedArrayBuffer(4));
  const sched = new Float64Array(blocks), rel = new Float64Array(blocks), seen = new Float64Array(blocks * K), allDone = new Float64Array(blocks);
  let stuck = null;
  for (let b = 0; b < blocks && !stuck; b++) {
    sched[b] = t0 + b * periodMs;
    for (let d = sched[b] - abs(); d > 0; d = sched[b] - abs()) Atomics.wait(sleepSab, 0, 0, d);
    rel[b] = abs();
    for (let k = 0; k < K; k++) { ins[k].set(src); Atomics.store(ctrls[k], 0, b + 1); Atomics.notify(ctrls[k], 0); }
    for (let k = 0; k < K; k++) {
      const c = ctrls[k];
      const give = abs() + 20000;
      for (;;) { const v = Atomics.load(c, 1); if (v === b + 1) break; if (abs() > give) { stuck = `plug-in ${k} did not answer block ${b} in 20 s`; break; } Atomics.wait(c, 1, v, 1000); }
      if (stuck) break;
      seen[b * K + k] = abs();
      dst.set(outs[k]);
    }
    allDone[b] = abs();
  }
  postMessage({ type: 'engine-done', K, blocks, periodMs, t0, sched, rel, seen, allDone, stuck }, [sched.buffer, rel.buffer, seen.buffer, allDone.buffer]);
};
