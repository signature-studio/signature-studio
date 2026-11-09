// Decision 0177 (srm1): the self time per function of a V8 .cpuprofile (node --cpu-prof), restricted to a time window if given, as a TSV: rank, function, self_ms, self%, cum%.
// usage: node cpuprof_top.mjs FILE.cpuprofile [TOPN=40] [FROM_US=0] [TO_US=inf]   (wasm functions carry names when the wasm is linked with --profiling-funcs)
import fs from 'node:fs';
const [, , file, topArg, fromArg, toArg] = process.argv;
const p = JSON.parse(fs.readFileSync(file, 'utf8'));
const top = +(topArg || 40), from = +(fromArg || 0), to = toArg ? +toArg : Infinity;
const byId = new Map(p.nodes.map((n) => [n.id, n]));
const self = new Map();
let t = p.startTime, total = 0;
for (let i = 0; i < p.samples.length; i++) {
  const dt = p.timeDeltas[i];
  t += dt;
  const rel = t - p.startTime;
  if (rel < from || rel > to) continue;
  const n = byId.get(p.samples[i]);
  const cf = n.callFrame;
  const key = cf.functionName || '(anonymous)';
  self.set(key, (self.get(key) || 0) + dt);
  total += dt;
}
const rows = [...self.entries()].sort((a, b) => b[1] - a[1]).slice(0, top);
console.log('rank\tfunction\tself_ms\tself_pct\tcum_pct');
let cum = 0;
rows.forEach(([k, v], i) => { cum += v; console.log(`${i + 1}\t${k}\t${(v / 1000).toFixed(1)}\t${(100 * v / total).toFixed(2)}\t${(100 * cum / total).toFixed(2)}`); });
console.log(`# total sampled ms in window: ${(total / 1000).toFixed(1)}`);
