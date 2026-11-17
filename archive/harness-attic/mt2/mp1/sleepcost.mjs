// mt2: what a guest Sleep(1) costs when the thread is a real Worker: Atomics.wait with a 1 ms timeout, N times; wall time per call and the Worker's CPU time per call (process.threadCpuUsage in the worker).
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
if (!isMainThread) {
  const ia = new Int32Array(workerData.sab);
  const N = workerData.n, ms = workerData.ms;
  const c0 = process.threadCpuUsage(); const t0 = performance.now();
  for (let i = 0; i < N; i++) Atomics.wait(ia, 0, 0, ms);
  const c1 = process.threadCpuUsage(); const t1 = performance.now();
  parentPort.postMessage({ ms, n: N, wall_per_call_ms: (t1 - t0) / N, cpu_per_call_us: ((c1.user - c0.user) + (c1.system - c0.system)) / N, cpu_user_us: (c1.user - c0.user) / N, cpu_sys_us: (c1.system - c0.system) / N });
} else {
  const out = [];
  for (const ms of [1, 0.2]) {
    const sab = new SharedArrayBuffer(64);
    const w = new Worker(new URL(import.meta.url), { workerData: { sab, n: ms === 1 ? 4000 : 4000, ms } });
    out.push(await new Promise((r) => w.once('message', r)));
    await w.terminate();
  }
  console.log(JSON.stringify(out, null, 1));
}
