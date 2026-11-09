// mt2 mp1 in a browser: the "engine Worker". Compiles mp1.wasm once, creates the shared Memory, starts the helper Worker (nested) with the same Module + Memory, measures.
const KERNEL_SP = [0xE0000, 0xF0000];
function boot(module, memory, idx) {
  const t0 = performance.now();
  const inst = new WebAssembly.Instance(module, { env: { memory } });
  const ex = inst.exports;
  ex.__wasm_init_tls(ex.tls_block_addr(idx));
  if (idx > 0) ex.__stack_pointer.value = KERNEL_SP[idx - 1];
  return { ex, ms: performance.now() - t0 };
}
self.onmessage = async (ev) => {
  const { url, N, LAT, SPINS, reps } = ev.data;
  const res = { url, N, ua: navigator.userAgent, hc: navigator.hardwareConcurrency, isolated: self.crossOriginIsolated };
  try {
    const bytes = await (await fetch(url)).arrayBuffer();
    const t0 = performance.now();
    const module = await WebAssembly.compile(bytes);
    res.compile_ms = performance.now() - t0;
    const memory = new WebAssembly.Memory({ initial: 16, maximum: 1024, shared: true });
    const me = boot(module, memory, 0); const ex = me.ex;
    const alone = [];
    for (let i = 0; i < reps; i++) { const t = performance.now(); const r = ex.kernel(BigInt(N)); alone.push(performance.now() - t); if (i === 0) res.ref = r.toString(); }
    res.main_alone_ms = alone;
    const spawnT = performance.now();
    const w = new Worker('/mp1-help.js');
    const ready = await new Promise((r) => { w.onmessage = (e) => r(e.data); w.postMessage({ module, memory, idx: 1 }); });
    res.spawn_to_ready_ms = performance.now() - spawnT;
    res.helper_instantiate_ms = ready.instantiate_ms;
    const pct = (xs, p) => xs[Math.min(xs.length - 1, Math.floor(xs.length * p))];
    res.latency = {};
    for (const spin of SPINS) {
      ex.set_spin(0, spin);
      for (let i = 0; i < 200; i++) { ex.job_post(0, 0, 0n); ex.job_wait(0); }
      const dts = [];
      for (let i = 0; i < LAT; i++) { const s = performance.now(); ex.job_post(0, 0, 0n); ex.job_wait(0); dts.push((performance.now() - s) * 1000); }
      dts.sort((x, y) => x - y);
      res.latency['spin' + spin] = { n: LAT, p50_us: pct(dts, 0.5), p90_us: pct(dts, 0.9), p99_us: pct(dts, 0.99), p999_us: pct(dts, 0.999), max_us: dts[dts.length - 1] };
    }
    ex.set_spin(0, 0);
    const ha = [];
    for (let i = 0; i < reps; i++) { const s = performance.now(); ex.job_post(0, 1, BigInt(N)); const r = ex.job_wait(0); ha.push({ ms: performance.now() - s, ok: r.toString() === res.ref }); }
    res.helper_alone = ha;
    const both = [];
    for (let i = 0; i < reps; i++) {
      const s = performance.now(); ex.job_post(0, 1, BigInt(N)); const m = ex.kernel(BigInt(N)); const mm = performance.now() - s; const h = ex.job_wait(0);
      both.push({ main_ms: mm, total_ms: performance.now() - s, main_ok: m.toString() === res.ref, helper_ok: h.toString() === res.ref });
    }
    res.both = both;
    ex.job_exit(0);
    w.terminate();
  } catch (e) { res.error = String(e && e.stack || e); }
  self.postMessage(res);
};
