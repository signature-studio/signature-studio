// R5-6g: PE facts of the DLLs given on the command line (read only). usage: node evidence/R5-6g/facts.mjs FILE...
// Prints per file: size, SHA-256, ImageBase, SizeOfImage, and the DLL names of its import directory (the import closure).
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

function sections(b, e) {
  const n = b.readUInt16LE(e + 6);
  const osz = b.readUInt16LE(e + 20);
  const out = [];
  for (let i = 0; i < n; i++) {
    const s = e + 24 + osz + 40 * i;
    out.push({ va: b.readUInt32LE(s + 12), vs: b.readUInt32LE(s + 8), raw: b.readUInt32LE(s + 20), rs: b.readUInt32LE(s + 16) });
  }
  return out;
}

for (const f of process.argv.slice(2)) {
  const b = fs.readFileSync(f);
  const e = b.readUInt32LE(0x3c);
  const opt = e + 24;
  const secs = sections(b, e);
  const off = (rva) => {
    for (const s of secs) if (rva >= s.va && rva < s.va + Math.max(s.vs, s.rs)) return rva - s.va + s.raw;
    throw new Error(`rva ${rva.toString(16)} not in a section`);
  };
  const imp = b.readUInt32LE(opt + 112 + 8);
  const dlls = [];
  if (imp) {
    for (let d = off(imp); ; d += 20) {
      const nameRva = b.readUInt32LE(d + 12);
      if (!b.readUInt32LE(d) && !nameRva && !b.readUInt32LE(d + 16)) break;
      let o = off(nameRva);
      let z = o;
      while (b[z]) z++;
      dlls.push(b.toString('latin1', o, z));
    }
  }
  const sha = crypto.createHash('sha256').update(b).digest('hex');
  console.log(`${path.basename(f)}`);
  console.log(`  bytes ${b.length}  sha256 ${sha}`);
  console.log(`  ImageBase 0x${Number(b.readBigUInt64LE(opt + 24)).toString(16)}  SizeOfImage ${b.readUInt32LE(opt + 56)}`);
  console.log(`  import DLLs (${dlls.length}): ${dlls.join(', ')}`);
}
