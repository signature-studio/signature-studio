// A = this Worker (blocking Atomics.wait), B = the Worker of pingasync.js (Atomics.waitAsync in its event loop)
self.onmessage = async (ev) => {
  const n = ev.data.n; const sab = new SharedArrayBuffer(16); const ia = new Int32Array(sab);
  const b = new Worker('/pingasync.js');
  await new Promise((r) => { b.onmessage = () => r(); b.postMessage({ role: 'B', mode: 'async', sab, n }); });
  const dts = [];
  for (let i = 1; i <= n; i++) {
    const t = performance.now();
    Atomics.store(ia, 0, i); Atomics.notify(ia, 0);
    while (Atomics.load(ia, 1) !== i) Atomics.wait(ia, 1, Atomics.load(ia, 1), 100);
    dts.push((performance.now() - t) * 1000);
  }
  dts.sort((x, y) => x - y); const p = (q) => dts[Math.floor(dts.length * q)];
  b.terminate();
  self.postMessage({ ua: navigator.userAgent, n, mode: 'A blocking -> B waitAsync -> A blocking', p50_us: p(0.5), p90_us: p(0.9), p99_us: p(0.99), p999_us: p(0.999), max_us: dts[dts.length - 1] });
};
