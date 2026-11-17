// R4-5e: the wasm functions of one profile run that contain memory.atomic.notify (from the table of r45e-wasmfuncs.awk): their self time (s, all profile files of the run), largest first. Reads only; writes text to stdout.
// Usage: node tools/verify-kit/tools/r45e-fnflags.mjs <table.tsv> <run dir>
import fs from 'node:fs';
const [,, tsv, dir, ...names] = process.argv;
const info = new Map();
for (const l of fs.readFileSync(tsv, 'utf8').trim().split('\n').slice(1)) { const [i, , instr, w, n, c, calls] = l.split('\t'); info.set(i, { instr: +instr, w: +w, n: +n, c: +c, calls: +calls }); }
const self = new Map();
for (const f of fs.readdirSync(dir).filter((x) => /^r45e-p3-w\d+\.cpuprofile$/.test(x))) {
  const p = JSON.parse(fs.readFileSync(`${dir}/${f}`, 'utf8'));
  const nodes = new Map(p.nodes.map((n) => [n.id, n]));
  for (let i = 0; i < p.samples.length; i++) { const w = i + 1 < p.timeDeltas.length ? p.timeDeltas[i + 1] : 0; const nm = nodes.get(p.samples[i]).callFrame.functionName; self.set(nm, (self.get(nm) ?? 0) + w); }
}
const notifyFns = [...info].filter(([, v]) => v.n > 0).map(([k]) => k);
const rows = notifyFns.map((k) => [k, info.get(k), (self.get(`wasm-function[${k}]`) ?? 0) / 1e6]).sort((a, b) => b[2] - a[2]).slice(0, 8);
console.log(dir.split('/').pop(), 'notify functions:', notifyFns.length, 'self s (top 8):');
for (const [k, v, s] of rows) console.log(k.padStart(7), `instr ${v.instr} calls ${v.calls}`.padEnd(24), s.toFixed(2));
