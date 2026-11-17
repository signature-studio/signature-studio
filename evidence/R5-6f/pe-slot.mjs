// R5-6f: the call slot of PEQ2 (RVA 0x3503A0) in the raw DLL: section, raw bytes, base relocations, rebased value. Read only.
// usage: node evidence/R5-6f/pe-slot.mjs PATH_TO_fruity_parametric_eq2_x64.dll
import fs from 'node:fs';

const b = fs.readFileSync(process.argv[2]);
const e = b.readUInt32LE(0x3c);
const opt = e + 24;
const n = b.readUInt16LE(e + 6);
const osz = b.readUInt16LE(e + 20);
const secs = [];
for (let i = 0; i < n; i++) {
  const s = e + 24 + osz + 40 * i;
  secs.push({ name: b.toString('latin1', s, s + 8).replace(/\0.*/, ''), vs: b.readUInt32LE(s + 8), va: b.readUInt32LE(s + 12), rs: b.readUInt32LE(s + 16), raw: b.readUInt32LE(s + 20) });
}
const sec = (rva) => secs.find((s) => rva >= s.va && rva < s.va + Math.max(s.vs, s.rs));
const off = (rva) => { const s = sec(rva); return rva - s.va + s.raw; };
const ibase = Number(b.readBigUInt64LE(opt + 24));
const slot = 0x3503a0;
const s = sec(slot);
console.log(`ImageBase 0x${ibase.toString(16)}`);
console.log(`slot RVA 0x${slot.toString(16)} section ${s.name} file offset 0x${off(slot).toString(16)}`);
console.log(`raw 8 bytes ${b.subarray(off(slot), off(slot) + 8).toString('hex')}`);
const raw = Number(b.readBigUInt64LE(off(slot)));
console.log(`raw value 0x${raw.toString(16)} = ImageBase + RVA 0x${(raw - ibase).toString(16)}`);
const rel = { rva: b.readUInt32LE(opt + 112 + 8 * 5), size: b.readUInt32LE(opt + 112 + 8 * 5 + 4) };
const hits = [];
for (let p = off(rel.rva); p < off(rel.rva) + rel.size;) {
  const pg = b.readUInt32LE(p), bs = b.readUInt32LE(p + 4);
  if (!bs) break;
  for (let q = p + 8; q < p + bs; q += 2) {
    const v = b.readUInt16LE(q);
    const r = pg + (v & 0xfff);
    if ((v >> 12) === 10 && r === slot) hits.push(`DIR64 at 0x${r.toString(16)}`);
  }
  p += bs;
}
console.log(`base relocation for the slot: ${hits.length ? hits.join(', ') : 'none'}`);
const base = 0x9000000;
console.log(`rebased to base 0x${base.toString(16)}: 0x${(raw - ibase + base).toString(16)}`);
const t = sec(0x35f220);
console.log(`0x35f220 is in section ${t.name} (0x${t.va.toString(16)}..0x${(t.va + t.vs).toString(16)})`);
