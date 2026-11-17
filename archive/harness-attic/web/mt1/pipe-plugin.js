// mt1: a "plugin Worker" for the pipeline prototype (decision 0173). It runs the REAL lifted OTT (the harness main(), unchanged wasm) and paces its process loop by a SharedArrayBuffer
// request/response slot: the block loop of the harness calls clock_time_get(CLOCK_THREAD_CPUTIME_ID) once before and once after each process() call; the first call blocks until the producer
// has sent the next block (Atomics.wait), the second posts the response. THE 32 KiB EXCHANGED ARE STAND-INS: OTT computes on the input that its own harness synthesises; what is real is the DSP
// (the lifted OTT process(), the same wasm as in the scaling test) and the transfer / wake-up / scheduling cost of the hand-over.
import { createHost } from './vst-host.js';
const FRAMES = 4096, BLK = FRAMES * 2 * 4, OFF_IN = 64, OFF_OUT = 64 + BLK;
let ctx = null;
self.onmessage = async (e) => {
  const m = e.data;
  if (m.type === 'prep') {
    const ctrl = new Int32Array(m.sab, 0, 16);
    const inp = new Float32Array(m.sab, OFF_IN, BLK / 4), out = new Float32Array(m.sab, OFF_OUT, BLK / 4);
    const scratch = new Float32Array(BLK / 4);
    const st = { loop: false, parity: 0, seen: 0, done: 0, blocks: m.blocks };
    const lines = [];
    const host = createHost({ files: m.files, args: m.args, env: m.env, print: (s) => lines.push(s), hooks: {
      line: (l) => { if (l.startsWith('RESULT init_cpu_ms')) st.loop = true; },
      clock: (id) => {
        if (!st.loop || id !== 3) return;
        if (st.parity === 0) {
          if (st.done === 0) postMessage({ type: 'loop-ready', id: m.id });
          for (;;) { const v = Atomics.load(ctrl, 0); if (v !== st.seen) { st.seen = v; break; } Atomics.wait(ctrl, 0, st.seen); }
          scratch.set(inp);                 // read the block
          st.parity = 1;
        } else {
          out.set(scratch);                 // write the block (stand-in for the plugin's output)
          Atomics.store(ctrl, 1, st.seen); Atomics.notify(ctrl, 1);
          st.parity = 0; st.done++;
          if (st.done >= st.blocks) st.loop = false;
        }
      } } });
    let inst;
    try { inst = await WebAssembly.instantiate(m.module, host.imports); } catch (err) { postMessage({ type: 'error', id: m.id, error: String(err) }); return; }
    host.attach(inst);
    ctx = { host, lines, st, id: m.id };
    postMessage({ type: 'ready', id: m.id });
  } else if (m.type === 'go') {
    let rc, err = null;
    const t0 = performance.now();
    try { rc = ctx.host.run(); } catch (x) { err = String(x && x.stack || x); }
    postMessage({ type: 'done', id: ctx.id, rc, err, done: ctx.st.done, ms: performance.now() - t0, lines: ctx.lines.filter((l) => /^RESULT/.test(l)).map((l) => l.slice(0, 300)) });
  }
};
