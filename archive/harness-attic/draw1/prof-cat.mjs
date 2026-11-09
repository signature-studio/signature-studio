// draw1: self time of a named V8 .cpuprofile by owner: FLEngine guest (f_<VA> < 0x1d00000 or fb_), module guests (dsp 0x2000000-, qfc 0x1d00000-, Blend2D wrapper 0x4700000-, FreeType 0x4b00000-, ILTools 0x6000000-), Blend2D upstream (bl::), runtime (everything else wasm), JavaScript, (program), (idle), (gc).
import fs from 'node:fs';
const f = process.argv[2];
const p = JSON.parse(fs.readFileSync(f, 'utf8'));
const byId = new Map(p.nodes.map((x) => [x.id, x]));
const self = new Map();
for (let i = 0; i < p.samples.length; i++) self.set(p.samples[i], (self.get(p.samples[i]) ?? 0) + (p.timeDeltas[i] ?? 0));
const cat = {}; let total = 0;
function owner(cf) {
  const n = cf.functionName;
  if (n === '(idle)') return null;
  if (n.startsWith('(')) return n;
  const m = /^f[b]?_([0-9a-f]+)$/.exec(n);
  if (m) { const va = parseInt(m[1], 16); if (va >= 0x6000000) return 'guest ILTools'; if (va >= 0x4b00000) return 'guest FreeType'; if (va >= 0x4700000) return 'guest Blend2D wrapper'; if (va >= 0x2000000 && va < 0x4700000) return va < 0x2000000 + 0x1000000 ? 'guest dsp_ippv2' : 'guest other module'; if (va >= 0x1d00000) return 'guest quickfontcache'; return 'guest FLEngine'; }
  if (/^bl::|^BL|blend2d/i.test(n)) return 'Blend2D upstream (bl::)';
  if (cf.url && !/wasm/.test(cf.url)) return 'JavaScript';
  if (/^wasm-function/.test(n)) return 'wasm (unnamed)';
  return 'runtime (C: fle_*, libc, ...)';
}
for (const [id, us] of self) { const o = owner(byId.get(id).callFrame); if (o === null) continue; cat[o] = (cat[o] ?? 0) + us; total += us; }
console.log(`non-idle total ${(total / 1e6).toFixed(2)} s`);
for (const [k, v] of Object.entries(cat).sort((a, b) => b[1] - a[1])) console.log(`${(v / 1000).toFixed(0).padStart(6)} ms ${(100 * v / total).toFixed(1).padStart(5)}%  ${k}`);
