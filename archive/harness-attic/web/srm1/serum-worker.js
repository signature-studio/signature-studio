// srm1 (decision 0177): Worker that runs the Serum 2 harness wasm (lift/harness/vst3a/build_vst3a_wasm.sh) once with the files it is given, in a browser Worker (Chrome).
// Same shape as mt1's vst-worker.js (decision 0173). All stdout lines are kept (the page parses "vst: block N cpu .. wall .." and "RESULT" lines).
import { createHost } from './vst-host.js';
let ctx = null;
self.onmessage = async (e) => {
  const m = e.data;
  if (m.type === 'prep') {
    const lines = [];
    const host = createHost({ files: m.files, args: m.args, env: m.env, print: (s) => lines.push(s), hooks: { cpuLabel: 'wall(performance.now) in a Worker' } });
    const t0 = performance.now();
    let inst;
    try { inst = await WebAssembly.instantiate(m.module, host.imports); } catch (err) { postMessage({ type: 'error', id: m.id, error: String(err) }); return; }
    host.attach(inst);
    ctx = { host, lines, instMs: performance.now() - t0, flag: m.flag ? new Int32Array(m.flag) : null, id: m.id };
    postMessage({ type: 'ready', id: m.id, instMs: ctx.instMs, memBytes: inst.exports.memory.buffer.byteLength });
  } else if (m.type === 'go') {
    if (ctx.flag) Atomics.wait(ctx.flag, 0, 0);
    const t0 = performance.now();
    let rc, err = null;
    try { rc = ctx.host.run(); } catch (x) { err = String(x && x.stack || x); }
    const t1 = performance.now();
    postMessage({ type: 'done', id: ctx.id, rc, err, runMs: t1 - t0, startedAt: t0 + performance.timeOrigin, endedAt: t1 + performance.timeOrigin, lines: ctx.lines, memBytes: ctx.host.memBytes(), missing: ctx.host.missing() });
  }
};
