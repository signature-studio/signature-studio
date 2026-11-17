// R5-6f §7.4 (H6): do the real-module images of the product (the bases in lift/harness/b2d2/modules.tsv) and PEQ2 (base 0x9000000) overlap?
// usage (from the repository root): node evidence/R5-6f/overlap.mjs HOST_ROOT
// HOST_ROOT is the r56e host (the lift/ files of the 10 modules and PEQ2). Read only.
import fs from 'node:fs';
import path from 'node:path';

const host = process.argv[2];
const rows = fs.readFileSync('lift/harness/b2d2/modules.tsv', 'utf8').split('\n').filter((l) => l.trim() !== '')
  .map((l) => l.split('\t'));
const mods = rows.map((r) => ({ name: r[0], file: path.join(host, r[1]), base: Number(r[2]) }));
mods.push({ name: 'fruity_parametric_eq2_x64.dll', file: path.join(host, 'lift/out-plugins/fruity_parametric_eq2_x64.dll'), base: 0x9000000 });

const sizeOfImage = (f) => {
  const b = fs.readFileSync(f);
  return b.readUInt32LE(b.readUInt32LE(0x3c) + 24 + 56);
};
const out = mods.map((m) => ({ ...m, size: sizeOfImage(m.file) }));
out.sort((a, b) => a.base - b.base);
for (const m of out) {
  console.log(`${m.name.padEnd(34)} base 0x${m.base.toString(16).padStart(7, '0')} SizeOfImage 0x${m.size.toString(16)} end 0x${(m.base + m.size).toString(16)}`);
}
let overlaps = 0;
for (let i = 0; i < out.length; i++) {
  for (let j = i + 1; j < out.length; j++) {
    const a = out[i], b = out[j];
    if (a.base < b.base + b.size && b.base < a.base + a.size) overlaps++;
  }
}
const last = out.reduce((x, y) => (x.base + x.size >= y.base + y.size ? x : y));
console.log(`modules: ${out.length}  overlapping pairs: ${overlaps}`);
console.log(`largest end: 0x${(last.base + last.size).toString(16)} (${last.name})`);
