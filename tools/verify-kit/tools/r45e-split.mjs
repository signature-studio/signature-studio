// R4-5e: the CPU and the blocked time ("(program)") of the Worker threads per profile file, split by what the sample's frame is: wasm functions that contain memory.atomic.notify (flag from r45e-wasmfuncs.awk), wasm functions with wait32 or cmpxchg (the GIL slow path), other wasm functions, JS, (idle). Times in s per profile file. Reads only; writes TSV to stdout.
// Usage: node tools/verify-kit/tools/r45e-split.mjs <table.tsv> <run dir> [<run dir> ...]  (the table of r45e-wasmfuncs.awk for the module the run used; the module's indices are per build)
import fs from 'node:fs';
const [,, tsv, ...dirs] = process.argv;
const flags = new Map();
for (const l of fs.readFileSync(tsv, 'utf8').trim().split('\n').slice(1)) {
  const [i, , instr, w, n, c, calls] = l.split('\t');
  flags.set(i, { gil: Number(w) + Number(c) > 0, notify: Number(n) > 0 });
}
for (const d of dirs) {
  const files = fs.readdirSync(d).filter((f) => /^r45e-p3-w\d+\.cpuprofile$/.test(f)).sort();
  console.log(d.split('/').pop());
  console.log('file\tprog_blk_s\tgil_s\tnotify_s\twasm_other_s\tjs_s\tidle_s');
  const tot = { prog: 0, gil: 0, notify: 0, other: 0, js: 0, idle: 0 };
  for (const f of files) {
    const p = JSON.parse(fs.readFileSync(`${d}/${f}`, 'utf8'));
    const nodes = new Map(p.nodes.map((n) => [n.id, n]));
    const s = { prog: 0, gil: 0, notify: 0, other: 0, js: 0, idle: 0 };
    for (let i = 0; i < p.samples.length; i++) {
      const w = i + 1 < p.timeDeltas.length ? p.timeDeltas[i + 1] : 0;
      const cf = nodes.get(p.samples[i]).callFrame;
      const name = cf.functionName;
      const m = /^wasm-function\[(\d+)\]$/.exec(name);
      if (name === '(idle)') s.idle += w;
      else if (name === '(program)') s.prog += w;
      else if (m) {
        const fl = flags.get(m[1]);
        if (fl?.notify) s.notify += w;
        else if (fl?.gil) s.gil += w;
        else s.other += w;
      } else s.js += w;
    }
    for (const k of Object.keys(tot)) tot[k] += s[k];
    const r = (x) => (x / 1e6).toFixed(1);
    console.log(`${f}\t${r(s.prog)}\t${r(s.gil)}\t${r(s.notify)}\t${r(s.other)}\t${r(s.js)}\t${r(s.idle)}`);
  }
  const r = (x) => (x / 1e6).toFixed(1);
  console.log(`TOTAL\t${r(tot.prog)}\t${r(tot.gil)}\t${r(tot.notify)}\t${r(tot.other)}\t${r(tot.js)}\t${r(tot.idle)}`);
}
