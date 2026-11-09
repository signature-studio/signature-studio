// parity1: for the functions matching REGEX, their self time by caller (from the call tree of a .cpuprofile). usage: node prof-callers.mjs FILE REGEX [N]
import fs from 'node:fs';
const p = JSON.parse(fs.readFileSync(process.argv[2], 'utf8')); const re = new RegExp(process.argv[3]); const N = Number(process.argv[4] ?? 15);
const byId = new Map(p.nodes.map((x) => [x.id, x])); const parent = new Map();
for (const n of p.nodes) for (const c of n.children ?? []) parent.set(c, n.id);
const self = new Map(); for (let i = 0; i < p.samples.length; i++) self.set(p.samples[i], (self.get(p.samples[i]) ?? 0) + (p.timeDeltas[i] ?? 0));
const agg = new Map(); let tot = 0;
for (const [id, us] of self) { const n = byId.get(id); if (!re.test(n.callFrame.functionName)) continue; const par = byId.get(parent.get(id)); const k = n.callFrame.functionName + ' <- ' + (par ? par.callFrame.functionName : '?'); agg.set(k, (agg.get(k) ?? 0) + us); tot += us; }
console.log(`total ${(tot / 1000).toFixed(0)} ms`);
for (const [k, v] of [...agg].sort((a, b) => b[1] - a[1]).slice(0, N)) console.log(`${(v / 1000).toFixed(0).padStart(7)} ms  ${k}`);
