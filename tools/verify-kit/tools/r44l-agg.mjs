// R4-4l: the n runs of the P2 profile (out/r44l-<N>/ for the run numbers given) as one table: for the functions that are in the self-time top 10 of any of those runs, the self % of each run, the median of the runs, the inclusive % median, and the Worker CPU (ms/s, from report.json profs[0]: cpuS.worker / wallS) of each run. Reads only; writes markdown to stdout.
// Usage: node tools/verify-kit/tools/r44l-agg.mjs <run numbers, comma separated, e.g. 1,3,4>
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const runs = (process.argv[2] ?? '').split(',').filter(Boolean).map(Number);
if (runs.length === 0) { process.stderr.write('usage: node tools/verify-kit/tools/r44l-agg.mjs <run numbers, e.g. 1,3,4>\n'); process.exit(2); }
const kit = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SPECIAL = ['(program)', '(idle)', '(garbage collector)', '(root)'];
const keyOf = (cf) => [cf.functionName || '(anonymous)', cf.url || '', cf.lineNumber ?? -1, cf.columnNumber ?? -1].join('\u0001');
const label = (fn) => { const m = /^wasm-function\[(\d+)\]$/.exec(fn); return m ? { idx: Number(m[1]) } : { idx: null }; };

const per = [];
for (const n of runs) {
  const dir = path.join(kit, 'out', `r44l-${n}`);
  const files = fs.readdirSync(dir).filter((x) => /-w\d+\.cpuprofile$/.test(x)).sort();
  if (files.length !== 1) throw new Error(`run ${n}: expected one Worker profile, found ${files.length}`);
  const p = JSON.parse(fs.readFileSync(path.join(dir, files[0]), 'utf8'));
  const report = JSON.parse(fs.readFileSync(path.join(dir, 'report.json'), 'utf8'));
  const rep = Array.isArray(report) ? report[0] : report;
  const prof = rep.profs?.[0];
  const nodes = new Map(p.nodes.map((x) => [x.id, x]));
  const parent = new Map();
  for (const x of p.nodes) for (const c of x.children ?? []) parent.set(c, x.id);
  const total = p.timeDeltas.reduce((a, b) => a + b, 0);
  const self = new Map(); const incl = new Map(); const meta = new Map();
  const keyCache = new Map();
  const keyFor = (id) => { if (!keyCache.has(id)) { const cf = nodes.get(id).callFrame; keyCache.set(id, keyOf(cf)); meta.set(keyOf(cf), cf); } return keyCache.get(id); };
  p.samples.forEach((sid, i) => {
    const dt = p.timeDeltas[i];
    const k = keyFor(sid);
    self.set(k, (self.get(k) ?? 0) + dt);
    const seen = new Set();
    let id = sid;
    while (id !== undefined) { const kk = keyFor(id); if (!seen.has(kk)) { seen.add(kk); incl.set(kk, (incl.get(kk) ?? 0) + dt); } id = parent.get(id); }
  });
  const ranked = [...self.entries()].filter(([k]) => !SPECIAL.includes(k.split('\u0001')[0])).sort((a, b) => b[1] - a[1]);
  const special = Object.fromEntries(["(program)", "(idle)", "(garbage collector)"].map((x) => [x, [...self.entries()].filter(([k]) => k.split("\u0001")[0] === x).reduce((a, [, v]) => a + v, 0)]));
  per.push({ n, total, self, incl, ranked, meta, special, workerMsPerS: prof ? Math.round(prof.cpuS.worker / prof.wallS * 1000) : null, wallS: prof?.wallS ?? null, workerS: prof?.cpuS.worker ?? null, samples: p.samples.length });
}

const median = (xs) => { const s = [...xs].sort((a, b) => a - b); const m = Math.floor(s.length / 2); return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2; };
const pct = (x, t) => 100 * x / t;
const top = new Set();
for (const r of per) for (const [k] of r.ranked.slice(0, 10)) top.add(k);
const rows = [...top].map((k) => {
  const selfPct = per.map((r) => pct(r.self.get(k) ?? 0, r.total));
  const inclPct = per.map((r) => pct(r.incl.get(k) ?? 0, r.total));
  const cf = per.map((r) => r.meta.get(k)).find(Boolean);
  const rankIn = per.map((r) => { const i = r.ranked.findIndex(([kk]) => kk === k); return i < 0 ? '-' : String(i + 1); });
  return { k, fn: cf.functionName, idx: label(cf.functionName).idx, selfPct, inclPct, rankIn, medSelf: median(selfPct), medIncl: median(inclPct) };
}).sort((a, b) => b.medSelf - a.medSelf).slice(0, 10);

console.log(`# R4-4l: top 10 by the median self %, runs ${runs.join(', ')} (${per.length} runs)`);
console.log('');
console.log('| rank (median) | function | index | self % per run | self % median | incl % median | rank per run | where |');
console.log('|---|---|---|---|---|---|---|---|');
rows.forEach((r, i) => {
  const where = r.k.split('\u0001')[1]?.split('/').pop() ?? '';
  console.log(`| ${i + 1} | ${r.fn} | ${r.idx ?? '-'} | ${r.selfPct.map((x) => x.toFixed(2)).join(' / ')} | ${r.medSelf.toFixed(2)} | ${r.medIncl.toFixed(2)} | ${r.rankIn.join(' / ')} | ${where} |`);
});
console.log('');
console.log('| run | samples | profile total ms | Worker CPU s | wall s | Worker ms/s |');
console.log('|---|---|---|---|---|---|');
for (const r of per) console.log(`| r44l-${r.n} | ${r.samples} | ${(r.total / 1000).toFixed(1)} | ${r.workerS} | ${r.wallS} | ${r.workerMsPerS} |`);
const ms = per.map((r) => r.workerMsPerS);
console.log('');
const sp = (x) => median(per.map((r) => pct(r.special[x], r.total)));
console.log(`Worker ms/s median ${median(ms)} (range ${Math.min(...ms)}-${Math.max(...ms)}); special shares (median of the runs): (program) ${sp("(program)").toFixed(2)} %, (idle) ${sp("(idle)").toFixed(2)} %, (garbage collector) ${sp("(garbage collector)").toFixed(2)} %`);
const extra = (process.argv[3] ?? '').split(',').filter(Boolean).map(Number);
if (extra.length > 0) {
  console.log('');
  console.log(`# R4-4l: the given function indices, runs ${runs.join(', ')}`);
  console.log('');
  console.log('| index | function | self % per run | self % median | incl % per run | incl % median |');
  console.log('|---|---|---|---|---|---|');
  for (const idx of extra) {
    const keys = per.map((r) => [...r.meta.keys()].find((k) => r.meta.get(k).functionName === `wasm-function[${idx}]`));
    const fn = (per.map((r, i) => (keys[i] ? r.meta.get(keys[i]).functionName : null)).find(Boolean)) ?? `wasm-function[${idx}]`;
    const selfP = per.map((r, i) => pct(keys[i] ? r.self.get(keys[i]) ?? 0 : 0, r.total));
    const inclP = per.map((r, i) => pct(keys[i] ? r.incl.get(keys[i]) ?? 0 : 0, r.total));
    console.log(`| ${idx} | ${fn} | ${selfP.map((x) => x.toFixed(2)).join(' / ')} | ${median(selfP).toFixed(2)} | ${inclP.map((x) => x.toFixed(2)).join(' / ')} | ${median(inclP).toFixed(2)} |`);
  }
}
