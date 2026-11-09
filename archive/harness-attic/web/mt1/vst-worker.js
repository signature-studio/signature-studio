// Worker of the scaling test: instantiates the shared WebAssembly.Module, waits at the start barrier, runs the OTT harness main() once, reports lines and per-block times.
import { createHost } from './vst-host.js';
let ctx = null;
self.onmessage = async (e) => {
  const m = e.data;
  if (m.type === 'prep') {
    const lines = [];
    const host = createHost({ files: m.files, args: m.args, env: m.env, print: (s) => lines.push(s) });
    const t0 = performance.now();
    let inst;
    try { inst = await WebAssembly.instantiate(m.module, host.imports); } catch (err) { postMessage({ type: 'error', id: m.id, error: String(err) }); return; }
    host.attach(inst);
    ctx = { host, lines, instMs: performance.now() - t0, flag: new Int32Array(m.flag), id: m.id, wasmPages: inst.exports.memory.buffer.byteLength / 65536 };
    postMessage({ type: 'ready', id: m.id, instMs: ctx.instMs, memBytes: inst.exports.memory.buffer.byteLength });
  } else if (m.type === 'go') {
    Atomics.wait(ctx.flag, 0, 0);       // released by the page (Atomics.store + notify) when every worker is ready
    const t0 = performance.now();
    let rc, err = null;
    try { rc = ctx.host.run(); } catch (x) { err = String(x && x.stack || x); }
    const t1 = performance.now();
    const blocks = [];
    const keep = [];
    for (const l of ctx.lines) {
      const mb = /^vst: block (\d+)\s+cpu ([\d.]+) ms\s+wall ([\d.]+) ms/.exec(l);
      if (mb) blocks[+mb[1]] = +mb[3];
      else if (/^RESULT|^vst: LoadLibraryW|^vst: factory\.\.initialize|setActive|^vst: process/.test(l)) keep.push(l.slice(0, 400));
    }
    postMessage({ type: 'done', id: ctx.id, rc, err, runMs: t1 - t0, startedAt: t0 + performance.timeOrigin, endedAt: t1 + performance.timeOrigin, blocks, lines: keep, missing: ctx.host.missing(), memBytes: ctx.host ? undefined : 0 });
  }
};
