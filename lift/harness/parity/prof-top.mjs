// parity1: self-time summary of a V8 .cpuprofile of the engine Worker: by category (wasm / JavaScript / (program) / (garbage collector) / idle) and the top self-time functions. usage: node prof-top.mjs FILE [N]
import fs from 'node:fs';
const p = JSON.parse(fs.readFileSync(process.argv[2], 'utf8')); const N = Number(process.argv[3] ?? 25);
const byId = new Map(p.nodes.map((x) => [x.id, x])); const self = new Map();
for (let i = 0; i < p.samples.length; i++) self.set(p.samples[i], (self.get(p.samples[i]) ?? 0) + (p.timeDeltas[i] ?? 0));
const cat = {}, fn = new Map(); let total = 0, idle = 0;
for (const [id, us] of self) {
  const cf = byId.get(id).callFrame; const n = cf.functionName || '(anonymous)';
  if (n === '(idle)') { idle += us; continue; }
  total += us;
  const c = n.startsWith('(') ? n : /^wasm-function|^f_[0-9a-f]+$|^fb_/.test(n) || /wasm/.test(cf.url) ? 'wasm' : 'JavaScript';
  cat[c] = (cat[c] ?? 0) + us;
  const k = n + ' ' + (cf.url ? cf.url.split('/').pop() : '') + ':' + cf.lineNumber; fn.set(k, (fn.get(k) ?? 0) + us);
}
console.log(`non-idle ${(total / 1e6).toFixed(2)} s, idle ${(idle / 1e6).toFixed(2)} s`);
for (const [k, v] of Object.entries(cat).sort((a, b) => b[1] - a[1])) console.log(`${(v / 1000).toFixed(0).padStart(7)} ms ${(100 * v / total).toFixed(1).padStart(5)}%  ${k}`);
console.log('-- top self');
for (const [k, v] of [...fn].sort((a, b) => b[1] - a[1]).slice(0, N)) console.log(`${(v / 1000).toFixed(0).padStart(7)} ms ${(100 * v / total).toFixed(1).padStart(5)}%  ${k}`);
