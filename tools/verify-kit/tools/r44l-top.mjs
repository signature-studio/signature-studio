// R4-4l: the self-time top 10 of the play-w*.cpuprofile files of tools/verify-kit/out/r44l-*/ (one table per file), with the inclusive share and the callers of each of the 10. Reads only; writes markdown to stdout.
// Usage: node tools/verify-kit/tools/r44l-top.mjs [top N, default 10]
// Definitions (the same as r44-top.py, R4-4): self time of a function = the sum of timeDeltas (us) of the samples whose top frame is a node of that function; nodes are grouped by callFrame functionName + url + lineNumber + columnNumber. share = self time / total time of the profile (the sum of all timeDeltas), in %. inclusive = the sum of timeDeltas of the samples in which the function is on the stack (once per sample), in % of the total. callers = the parent functions of the top-frame samples of the function, by their self time, in %.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const top = Number(process.argv[2] ?? 10);
const kit = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const out = path.join(kit, 'out');
const dirs = fs.readdirSync(out).filter((d) => /^r44l-\d+$/.test(d)).sort((a, b) => Number(a.slice(5)) - Number(b.slice(5)));
const keyOf = (cf) => [cf.functionName || '(anonymous)', cf.url || '', cf.lineNumber ?? -1, cf.columnNumber ?? -1].join('\u0001');
const SPECIAL = ['(program)', '(idle)', '(garbage collector)', '(root)'];
const label = (k) => {
  const [fn, url, line, col] = k.split('\u0001');
  const m = /^wasm-function\[(\d+)\]$/.exec(fn);
  return { fn, where: url ? `${path.basename(url)}:${Number(line) + 1}` : '', index: m ? Number(m[1]) : null, col: Number(col) };
};

const rows = [];
for (const d of dirs) {
  const dir = path.join(out, d);
  for (const f of fs.readdirSync(dir).filter((x) => /-w\d+\.cpuprofile$/.test(x)).sort()) {
    const p = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
    const nodes = new Map(p.nodes.map((n) => [n.id, n]));
    const parent = new Map();
    for (const n of p.nodes) for (const c of n.children ?? []) parent.set(c, n.id);
    const total = p.timeDeltas.reduce((a, b) => a + b, 0);
    const self = new Map(); const incl = new Map(); const callers = new Map();
    const keyCache = new Map();
    const keyFor = (id) => { if (!keyCache.has(id)) keyCache.set(id, keyOf(nodes.get(id).callFrame)); return keyCache.get(id); };
    p.samples.forEach((sid, i) => {
      const dt = p.timeDeltas[i];
      const k = keyFor(sid);
      self.set(k, (self.get(k) ?? 0) + dt);
      const par = parent.get(sid);
      if (par !== undefined) { const pk = keyFor(par); const cm = callers.get(k) ?? new Map(); cm.set(pk, (cm.get(pk) ?? 0) + dt); callers.set(k, cm); }
      const seen = new Set();
      let id = sid;
      while (id !== undefined) { const kk = keyFor(id); if (!seen.has(kk)) { seen.add(kk); incl.set(kk, (incl.get(kk) ?? 0) + dt); } id = parent.get(id); }
    });
    const special = Object.fromEntries(SPECIAL.map((s) => [s, [...self.entries()].filter(([k]) => k.split('\u0001')[0] === s).reduce((a, [, v]) => a + v, 0)]));
    const ranked = [...self.entries()].filter(([k]) => !SPECIAL.includes(k.split('\u0001')[0])).sort((a, b) => b[1] - a[1]).slice(0, top);
    rows.push({ run: d, file: f, samples: p.samples.length, total, ranked, self, incl, callers, special });
  }
}

const pct = (x, t) => (100 * x / t).toFixed(2);
const ms = (x) => (x / 1000).toFixed(1);
console.log(`# R4-4l: self-time top ${top} per profile (out/r44l-*/play-w*.cpuprofile)`);
console.log('# share % = self time / total time of the profile; incl % = inclusive (samples with the function on the stack) / total; callers = parent functions of the top-frame samples, % of the function self time');
console.log('');
for (const r of rows) {
  console.log(`## ${r.run} ${r.file}: samples ${r.samples}, total ${ms(r.total)} ms`);
  console.log('| rank | function | index | where | self ms | self % | incl % | main callers (self-time share) |');
  console.log('|---|---|---|---|---|---|---|---|');
  r.ranked.forEach(([k, v], i) => {
    const l = label(k);
    const cm = r.callers.get(k);
    const cs = cm ? [...cm.entries()].sort((a, b) => b[1] - a[1]).slice(0, 2).map(([pk, pv]) => `${label(pk).fn} ${pct(pv, v)}`).join('; ') : '-';
    console.log(`| ${i + 1} | ${l.fn} | ${l.index ?? '-'} | ${l.where} | ${ms(v)} | ${pct(v, r.total)} | ${pct(r.incl.get(k) ?? 0, r.total)} | ${cs} |`);
  });
  console.log(`special: (program) ${pct(r.special['(program)'], r.total)} %, (idle) ${pct(r.special['(idle)'], r.total)} %, (garbage collector) ${pct(r.special['(garbage collector)'], r.total)} %`);
  console.log('');
}
