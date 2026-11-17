// parity1: self time per function of profile B minus profile A (e.g. a session minus its prelude). usage: node prof-diff.mjs A.cpuprofile B.cpuprofile [N]
import fs from 'node:fs';
function self(f) { const p = JSON.parse(fs.readFileSync(f, 'utf8')); const byId = new Map(p.nodes.map((x) => [x.id, x])); const m = new Map(); let tot = 0;
  for (let i = 0; i < p.samples.length; i++) { const cf = byId.get(p.samples[i]).callFrame; const n = cf.functionName || '(anonymous)'; if (n === '(idle)') continue; const us = p.timeDeltas[i] ?? 0; m.set(n, (m.get(n) ?? 0) + us); tot += us; }
  return { m, tot }; }
const A = self(process.argv[2]), B = self(process.argv[3]), N = Number(process.argv[4] ?? 30);
const d = [...B.m].map(([k, v]) => [k, v - (A.m.get(k) ?? 0)]).sort((a, b) => b[1] - a[1]);
const tot = B.tot - A.tot;
console.log(`B ${(B.tot / 1e6).toFixed(2)} s - A ${(A.tot / 1e6).toFixed(2)} s = ${(tot / 1e6).toFixed(2)} s (non-idle self time, profiler overhead included)`);
for (const [k, v] of d.slice(0, N)) console.log(`${(v / 1000).toFixed(0).padStart(7)} ms ${(100 * v / tot).toFixed(1).padStart(5)}%  ${k}`);
