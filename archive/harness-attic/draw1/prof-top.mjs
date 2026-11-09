// draw1: self time by function (V8 .cpuprofile), top N, and the share of the total. usage: node prof-top.mjs FILE [N]
import fs from 'node:fs';
const [f, n = '25'] = process.argv.slice(2);
const p = JSON.parse(fs.readFileSync(f, 'utf8'));
const byId = new Map(p.nodes.map((x) => [x.id, x]));
const self = new Map();
for (let i = 0; i < p.samples.length; i++) self.set(p.samples[i], (self.get(p.samples[i]) ?? 0) + (p.timeDeltas[i] ?? 0));
const fn = new Map(); let total = 0, idle = 0;
for (const [id, us] of self) { const cf = byId.get(id).callFrame; const k = cf.functionName || '(anon)'; if (k === '(idle)') { idle += us; continue; } total += us; fn.set(k, (fn.get(k) ?? 0) + us); }
console.log(`total non-idle ${(total / 1e6).toFixed(2)} s (idle ${(idle / 1e6).toFixed(2)} s)`);
for (const [k, v] of [...fn].sort((a, b) => b[1] - a[1]).slice(0, +n)) console.log(`${(v / 1000).toFixed(0).padStart(6)} ms ${(100 * v / total).toFixed(1).padStart(5)}%  ${k}`);
