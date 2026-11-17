// node harness for pingasync.js semantics (worker_threads): A = this thread (blocking), B = Worker with Atomics.waitAsync
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
if (!isMainThread) {
  const ia = new Int32Array(workerData.sab); const { n } = workerData; let last = 0; setInterval(() => {}, 1000);   // node ends a Worker whose only pending work is a waitAsync promise
  parentPort.postMessage('ready');
  while (true) {
    let cur = Atomics.load(ia, 0);
    while (cur === last) { const r = Atomics.waitAsync(ia, 0, last); if (r.async) await r.value; cur = Atomics.load(ia, 0); }
    last = cur; Atomics.store(ia, 1, cur); Atomics.notify(ia, 1);
    if (cur === n) break;
  }
  parentPort.postMessage('done');
} else {
  const sab = new SharedArrayBuffer(16); const ia = new Int32Array(sab); const n = Number(process.argv[2] || 20000);
  const w = new Worker(new URL(import.meta.url), { workerData: { sab, n } });
  await new Promise((r) => w.once('message', r));
  const dts = [];
  for (let i = 1; i <= n; i++) {
    const t = process.hrtime.bigint();
    Atomics.store(ia, 0, i); Atomics.notify(ia, 0);
    while (Atomics.load(ia, 1) !== i) Atomics.wait(ia, 1, Atomics.load(ia, 1), 100);
    dts.push(Number(process.hrtime.bigint() - t) / 1000);
  }
  dts.sort((a, b) => a - b); const p = (q) => dts[Math.floor(dts.length * q)];
  console.log(JSON.stringify({ node: process.version, n, mode: 'A blocking -> B waitAsync -> A blocking', p50_us: p(0.5), p90_us: p(0.9), p99_us: p(0.99), p999_us: p(0.999), max_us: dts[dts.length - 1] }));
  await new Promise((r) => w.once('message', r)); await w.terminate();
}
