// mt2 micro-prototype 1 (decision 0234, measurement only): node worker_threads harness for mp1.wasm (see mp1.c).
// The first thread is the "engine Worker" (it may block with Atomics.wait in node's main thread; a browser engine Worker may too), the second is the helper. Both instantiate the SAME WebAssembly.Module on the SAME shared Memory.
// usage: node mp1.mjs WASM [--n 20000000] [--lat 20000] [--spin 0,2000,200000] [--json OUT]
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import fs from 'node:fs';

const KERNEL_SP = [0xE0000, 0xF0000];   // shadow-stack tops of the helpers (the first thread keeps the default one)

function boot(module, memory, idx) {
  const t0 = performance.now();
  const inst = new WebAssembly.Instance(module, { env: { memory } });
  const ex = inst.exports;
  const size = ex.tls_block_size();
  if (ex.__tls_size.value > size) throw new Error('TLS block too small: ' + ex.__tls_size.value);
  ex.__wasm_init_tls(ex.tls_block_addr(idx));
  if (idx > 0) ex.__stack_pointer.value = KERNEL_SP[idx - 1];
  return { ex, ms: performance.now() - t0 };
}

if (!isMainThread) {
  const { module, memory, idx } = workerData;
  const b = boot(module, memory, idx);
  parentPort.postMessage({ ready: true, instantiate_ms: b.ms, t_ready: performance.timeOrigin + performance.now() });
  b.ex.helper_loop(idx - 1);   // returns after job_exit
  parentPort.postMessage({ exited: true });
} else {
  const a = process.argv.slice(2);
  const wasmFile = a[0];
  const opt = (n, d) => { const i = a.indexOf('--' + n); return i < 0 ? d : a[i + 1]; };
  const N = Number(opt('n', '20000000'));
  const LAT = Number(opt('lat', '20000'));
  const SPINS = opt('spin', '0,2000,200000').split(',').map(Number);
  const module = new WebAssembly.Module(fs.readFileSync(wasmFile));
  const memory = new WebAssembly.Memory({ initial: 16, maximum: 1024, shared: true });
  const res = { wasm: wasmFile, node: process.version, n: N, cpus: (await import('node:os')).cpus().length };
  const me = boot(module, memory, 0);
  const ex = me.ex;
  const pct = (xs, p) => xs[Math.min(xs.length - 1, Math.floor(xs.length * p))];
  // 1. the kernel alone on this thread: time and the reference result
  let t = performance.now();
  const ref = ex.kernel(BigInt(N));
  res.alone_ms = performance.now() - t;
  res.ref_result = ref.toString();
  // 2. start the helper
  const spawnT = performance.now();
  const w = new Worker(new URL(import.meta.url), { workerData: { module, memory, idx: 1 } });
  const ready = await new Promise((r) => w.once('message', r));
  res.spawn_to_ready_ms = performance.now() - spawnT;
  res.helper_instantiate_ms = ready.instantiate_ms;
  // 3. job latency (noop jobs, helper sleeping in memory.atomic.wait32 or spinning for `spin` iterations before it sleeps)
  res.latency = {};
  for (const spin of SPINS) {
    ex.set_spin(0, spin);
    const dts = [];
    for (let i = 0; i < 200; i++) { ex.job_post(0, 0, 0n); ex.job_wait(0); }   // warm up
    for (let i = 0; i < LAT; i++) {
      const s = process.hrtime.bigint();
      ex.job_post(0, 0, 0n);
      ex.job_wait(0);
      dts.push(Number(process.hrtime.bigint() - s) / 1000);
    }
    dts.sort((x, y) => x - y);
    res.latency['spin' + spin] = { n: LAT, p50_us: pct(dts, 0.5), p90_us: pct(dts, 0.9), p99_us: pct(dts, 0.99), p999_us: pct(dts, 0.999), max_us: dts[dts.length - 1], mean_us: dts.reduce((x, y) => x + y, 0) / dts.length };
  }
  ex.set_spin(0, 0);
  // 4. both threads run the kernel at the same time on the same memory: the helper's result and the time of each
  const both = [];
  for (let rep = 0; rep < 3; rep++) {
    const s0 = performance.now();
    ex.job_post(0, 1, BigInt(N));
    const mine = ex.kernel(BigInt(N));
    const mineMs = performance.now() - s0;
    const theirs = ex.job_wait(0);
    const totalMs = performance.now() - s0;
    both.push({ main_ms: mineMs, total_ms: totalMs, main_ok: mine === ref, helper_ok: theirs === ref, helper_result: theirs.toString() });
  }
  res.both = both;
  // 5. the helper alone (job_wait right after job_post, the main thread idle)
  const alone2 = [];
  for (let rep = 0; rep < 3; rep++) { const s0 = performance.now(); ex.job_post(0, 1, BigInt(N)); const r = ex.job_wait(0); alone2.push({ ms: performance.now() - s0, ok: r === ref }); }
  res.helper_alone = alone2;
  ex.job_exit(0);
  await new Promise((r) => w.once('message', r));
  await w.terminate();
  console.log(JSON.stringify(res, null, 1));
  const j = opt('json', ''); if (j) fs.writeFileSync(j, JSON.stringify(res, null, 1));
}
