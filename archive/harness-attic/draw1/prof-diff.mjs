// draw1: self time per function in profile A minus profile B (V8 .cpuprofile), top N by the difference. usage: node prof-diff.mjs A B [N]
import fs from 'node:fs';
const [a, b, n = '20'] = process.argv.slice(2);
const load = (f) => { const p = JSON.parse(fs.readFileSync(f, 'utf8')); const byId = new Map(p.nodes.map((x) => [x.id, x])); const m = new Map(); let tot = 0;
  for (let i = 0; i < p.samples.length; i++) { const cf = byId.get(p.samples[i]).callFrame; const k = cf.functionName || '(anon)'; const us = p.timeDeltas[i] ?? 0; if (k === '(idle)') continue; m.set(k, (m.get(k) ?? 0) + us); tot += us; } return { m, tot }; };
const A = load(a), B = load(b);
console.log(`A total ${(A.tot / 1e6).toFixed(2)} s, B total ${(B.tot / 1e6).toFixed(2)} s, A-B ${((A.tot - B.tot) / 1e6).toFixed(2)} s`);
const keys = new Set([...A.m.keys(), ...B.m.keys()]);
const rows = [...keys].map((k) => [k, (A.m.get(k) ?? 0) - (B.m.get(k) ?? 0), A.m.get(k) ?? 0, B.m.get(k) ?? 0]).sort((x, y) => y[1] - x[1]);
const d = A.tot - B.tot;
for (const [k, v, av, bv] of rows.slice(0, +n)) console.log(`${(v / 1000).toFixed(0).padStart(7)} ms ${(100 * v / d).toFixed(1).padStart(5)}% of A-B   A ${(av / 1000).toFixed(0)} ms  B ${(bv / 1000).toFixed(0)} ms  ${k}`);
