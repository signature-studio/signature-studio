// pw1 (decision 0186): ONE plug-in instance in ONE Worker that renders ON ITS OWN SCHEDULE into an output ring (decoupled, optionally phase-staggered) - the alternative to plugin-worker.js, where the engine releases
// all blocks in lock step. Block b of this instance is started at T0 + b*P + phi - ahead*P (phi = its phase, 0 <= phi < P; if it is late it starts at once) and its 32 KiB output goes into slot (b mod 4) of
// the ring; the counter ctrl[1] (blocks produced) is the only hand-over. The engine reader (ring-engine-worker.js) takes block b at T0 + (b+1+lead)*P and counts the instances that have not produced it.
// The harness main() is unchanged: the same clock-call trick as plugin-worker.js (first call of a block waits for the instance's own start time, second call publishes). Slot layout (bytes): 0..63 Int32 words
// (1 = blocks produced, 2 = go), 64..127 Float64 (T0 epoch ms, period, phi, ahead), 128 stand-in input, then 4 output slots.
import { createHost as createSerumHost } from '../srm1/vst-host.js';
import { createHost as createOttHost } from '../mt1/vst-host.js';
const FRAMES = 4096, BLK = FRAMES * 2 * 4, NRING = 4, OFF_IN = 128, OFF_OUT = 128 + BLK;
const abs = () => performance.timeOrigin + performance.now();
let ctx = null;
self.onmessage = async (e) => {
  const m = e.data;
  if (m.type === 'prep') {
    const paced = !!m.sab;
    const ctrl = paced ? new Int32Array(m.sab, 0, 16) : null, par = paced ? new Float64Array(m.sab, 64, 8) : null;
    const inp = paced ? new Float32Array(m.sab, OFF_IN, BLK / 4) : null;
    const outs = paced ? Array.from({ length: NRING }, (_, s) => new Float32Array(m.sab, OFF_OUT + s * BLK, BLK / 4)) : null;
    const scratch = new Float32Array(BLK / 4), sleepW = new Int32Array(new SharedArrayBuffer(4));
    const st = { loop: false, armed: false, parity: 0, done: 0, blocks: m.blocks, warmT: [], t0: 0, T0: 0, P: 0, phi: 0, ahead: 0 };
    const T = new Float64Array(m.blocks * 4);
    const lines = [];
    const keep = (l) => (/^RESULT|^vst: (block|DllMain|factory|IComponent::setState|IEditController|process returned|LoadLibraryW)/.test(l) ? lines.push(l.slice(0, 260)) : 0);
    const pace = () => {
      if (!paced && m.warmMs && st.armed) {
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
        if (st.done === 0) {
          postMessage({ type: 'loop-ready', id: m.id });
          while (Atomics.load(ctrl, 2) === 0) Atomics.wait(ctrl, 2, 0);
          st.T0 = par[0]; st.P = par[1]; st.phi = par[2]; st.ahead = par[3];
        }
        const target = st.T0 + st.done * st.P + st.phi - st.ahead * st.P;
        for (let d = target - abs(); d > 0; d = target - abs()) Atomics.wait(sleepW, 0, 0, d);
        T[i + 1] = abs();
        scratch.set(inp);
        T[i + 2] = abs();
        st.parity = 1;
      } else {
        T[i + 3] = abs();
        outs[st.done % NRING].set(scratch);
        Atomics.store(ctrl, 1, st.done + 1); Atomics.notify(ctrl, 1);
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
