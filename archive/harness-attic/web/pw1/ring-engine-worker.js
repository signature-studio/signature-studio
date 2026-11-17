// pw1 (decision 0186): the READER of the decoupled output rings (ring-plugin-worker.js). It does not release or wait for anything: at T0 + (b+1+lead)*P it takes block b from every instance's ring if the instance
// has produced it (counter ctrl[1] >= b+1) and counts an under-run (miss) for every instance that has not. lead = 1 means one period of output delay (the same total delay as the A2 count of the lock-step engine).
const FRAMES = 4096, BLK = FRAMES * 2 * 4, NRING = 4, OFF_OUT = 128 + BLK;
const abs = () => performance.timeOrigin + performance.now();
self.onmessage = (e) => {
  const { sabs, blocks, periodMs, t0, lead } = e.data;
  const K = sabs.length;
  const ctrls = sabs.map((s) => new Int32Array(s, 0, 16));
  const outs = sabs.map((s) => Array.from({ length: NRING }, (_, r) => new Float32Array(s, OFF_OUT + r * BLK, BLK / 4)));
  const dst = new Float32Array(BLK / 4), sleepSab = new Int32Array(new SharedArrayBuffer(4));
  const readAt = new Float64Array(blocks), miss = new Uint8Array(blocks * K), prod = new Int32Array(blocks * K);
  for (let b = 0; b < blocks; b++) {
    const E = t0 + (b + 1 + lead) * periodMs;
    for (let d = E - abs(); d > 0; d = E - abs()) Atomics.wait(sleepSab, 0, 0, d);
    readAt[b] = abs();
    for (let k = 0; k < K; k++) {
      const p = Atomics.load(ctrls[k], 1);
      prod[b * K + k] = p;
      if (p >= b + 1) dst.set(outs[k][b % NRING]); else miss[b * K + k] = 1;
    }
  }
  postMessage({ type: 'ring-done', K, blocks, periodMs, t0, lead, readAt, miss, prod }, [readAt.buffer, miss.buffer, prod.buffer]);
};
