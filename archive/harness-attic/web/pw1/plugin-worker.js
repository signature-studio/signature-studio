// pw1 (decision 0186): ONE plug-in instance in ONE Worker, paced block by block by an "engine" Worker over a SharedArrayBuffer slot (the design of decision 0173 / mt1 pipe-plugin.js).
// kind 'serum': the lifted Serum2.vst3 (hybrid or C-path wasm of lift/harness/vst3a, host of srm1/vst-host.js). kind 'ott': the lifted OTT (mt1 host). The harness main() runs UNCHANGED:
// its block loop calls the CPU clock once before and once after each process(). Serum: the import env.srm1_thread_cpu_s (a Worker answers performance.now()); OTT: clock_time_get(3).
// After 'RESULT init_cpu_ms' (init, state, setActive done) the FIRST clock call of a block blocks (Atomics.wait) until the engine has published the next block and copies the 32 KiB input;
// the SECOND posts the response (32 KiB output copied) and wakes the engine. THE 32 KiB EXCHANGED ARE STAND-INS (an instrument has no audio input; OTT computes on the input its harness
// synthesises): what is real is the lifted process() and the hand-over / wake-up / scheduling cost. Without a slot (m.sab == null) the harness runs free (warm-up instance).
// Per block this Worker records (epoch ms, performance.timeOrigin + performance.now()): hook entered, request seen, process start, process end.
import { createHost as createSerumHost } from '../srm1/vst-host.js';
import { createHost as createOttHost } from '../mt1/vst-host.js';
const FRAMES = 4096, BLK = FRAMES * 2 * 4, OFF_IN = 64, OFF_OUT = 64 + BLK;
const abs = () => performance.timeOrigin + performance.now();
let ctx = null;
self.onmessage = async (e) => {
  const m = e.data;
  if (m.type === 'prep') {
    const paced = !!m.sab;
    const ctrl = paced ? new Int32Array(m.sab, 0, 16) : null;
    const inp = paced ? new Float32Array(m.sab, OFF_IN, BLK / 4) : null, out = paced ? new Float32Array(m.sab, OFF_OUT, BLK / 4) : null;
    const scratch = new Float32Array(BLK / 4);
    const st = { loop: false, armed: false, parity: 0, seen: 0, done: 0, blocks: m.blocks, warmT: [], t0: 0 };
    const T = new Float64Array(m.blocks * 4);
    const lines = [];
    const keep = (l) => (/^RESULT|^vst: (block|DllMain|factory|IComponent::setState|IEditController|process returned|LoadLibraryW)/.test(l) ? lines.push(l.slice(0, 260)) : 0);
    const pace = () => {
      if (!paced && m.warmMs && st.armed) {   // throw-away (warm-up) instance: free-running blocks for warmMs of wall time, then abandoned by an exception out of the harness
        const t = performance.now();
        if (!st.t0) st.t0 = t;
        if (st.parity === 0) st.warmT.push(t);
        st.parity ^= 1;
        if (t - st.t0 > m.warmMs && st.parity === 1) throw new Error('pw1-warm-done');
        return;
      }
      if (!st.loop) return;
      const i = st.done * 4;
      if (st.parity === 0) {
        T[i] = abs();
        if (st.done === 0) postMessage({ type: 'loop-ready', id: m.id });
        for (;;) { const v = Atomics.load(ctrl, 0); if (v !== st.seen) { st.seen = v; break; } Atomics.wait(ctrl, 0, st.seen); }
        T[i + 1] = abs();
        scratch.set(inp);
        T[i + 2] = abs();
        st.parity = 1;
      } else {
        T[i + 3] = abs();
        out.set(scratch);
        Atomics.store(ctrl, 1, st.seen); Atomics.notify(ctrl, 1);
        st.parity = 0; st.done++;
        if (st.done >= st.blocks) st.loop = false;
      }
    };
    const onLine = (l) => { keep(l); if (l.startsWith('RESULT init_cpu_ms')) { if (paced) st.loop = true; else st.armed = true; } };
    let host;
    if (m.kind === 'ott') host = createOttHost({ files: m.files, args: m.args, env: m.env, print: () => {}, hooks: { line: onLine, clock: (id) => { if (id === 3) pace(); } } });
    else {
      host = createSerumHost({ files: m.files, args: m.args, env: m.env, print: () => {}, hooks: { line: onLine, cpuLabel: 'wall(performance.now) in a Worker' } });
      host.imports.env.srm1_thread_cpu_s = () => { pace(); return performance.now() * 1e-3; };
    }
    const t0 = performance.now();
    let inst;
    try { inst = await WebAssembly.instantiate(m.module, host.imports); } catch (err) { postMessage({ type: 'error', id: m.id, error: String(err) }); return; }
    host.attach(inst);
    ctx = { host, lines, st, T, id: m.id, instMs: performance.now() - t0 };
    postMessage({ type: 'ready', id: m.id, instMs: ctx.instMs });
  } else if (m.type === 'go') {
    let rc, err = null;
    const t0 = performance.now();
    try { rc = ctx.host.run(); } catch (x) { err = String(x && x.stack || x); }
    postMessage({ type: 'done', id: ctx.id, rc, err, done: ctx.st.done, runMs: performance.now() - t0, lines: ctx.lines, T: ctx.T, warmT: ctx.st.warmT, memBytes: ctx.host.memBytes ? ctx.host.memBytes() : 0, missing: ctx.host.missing ? ctx.host.missing() : [] });
  }
};
