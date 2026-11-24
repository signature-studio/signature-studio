// feat-c1-hidpi/read-manifest: prints the RT_MANIFEST resources (type 24) of PE files, for the DPI declarations of FL's executables (Issue #1347: dpiAware, dpiAwareness,
// gdiScaling). Reads the PE resource tree with node only (no native code); nothing is written. Usage: node tools/feat-c1-hidpi/read-manifest.mjs <file.exe> [more files]
import fs from 'node:fs';

function readManifests(file) {
  const b = fs.readFileSync(file);
  const e_lfanew = b.readUInt32LE(0x3c);
  if (b.toString('latin1', e_lfanew, e_lfanew + 4) !== 'PE\0\0') throw new Error(`${file}: not a PE file`);
  const coff = e_lfanew + 4;
  const nsec = b.readUInt16LE(coff + 2);
  const optSize = b.readUInt16LE(coff + 16);
  const opt = coff + 20;
  const magic = b.readUInt16LE(opt);
  const ddBase = magic === 0x20b ? opt + 112 : opt + 96;
  const rsrcRva = b.readUInt32LE(ddBase + 2 * 8);
  const secs = [];
  const secBase = opt + optSize;
  for (let i = 0; i < nsec; i++) {
    const s = secBase + i * 40;
    secs.push({ vsize: b.readUInt32LE(s + 8), rva: b.readUInt32LE(s + 12), rawSize: b.readUInt32LE(s + 16), rawPtr: b.readUInt32LE(s + 20) });
  }
  const rvaToOff = (rva) => {
    for (const s of secs) {
      const size = Math.max(s.vsize, s.rawSize);
      if (rva >= s.rva && rva < s.rva + size) return rva - s.rva + s.rawPtr;
    }
    throw new Error(`${file}: rva ${rva.toString(16)} not mapped`);
  };
  if (!rsrcRva) return [];
  const rBase = rvaToOff(rsrcRva);
  const out = [];
  const walk = (off, path) => {
    const named = b.readUInt16LE(off + 12), ids = b.readUInt16LE(off + 14);
    for (let i = 0; i < named + ids; i++) {
      const e = off + 16 + i * 8;
      const nameOrId = b.readUInt32LE(e), data = b.readUInt32LE(e + 4);
      const id = (nameOrId & 0x80000000) ? `name@${nameOrId & 0x7fffffff}` : nameOrId;
      const p = [...path, id];
      if (data & 0x80000000) walk(rBase + (data & 0x7fffffff), p);
      else {
        const d = rBase + (data & 0x7fffffff);
        const dataRva = b.readUInt32LE(d), size = b.readUInt32LE(d + 4);
        const off2 = rvaToOff(dataRva);
        out.push({ path: p, bytes: b.subarray(off2, off2 + size) });
      }
    }
  };
  walk(rBase, []);
  return out.filter((x) => x.path[0] === 24).map((x) => ({ id: x.path, text: x.bytes.toString('utf8') }));
}

for (const f of process.argv.slice(2)) {
  const ms = readManifests(f);
  console.log(`== ${f}: ${ms.length} RT_MANIFEST entr${ms.length === 1 ? 'y' : 'ies'}`);
  for (const m of ms) console.log(`-- resource ${JSON.stringify(m.id)} (${m.text.length} chars)\n${m.text}`);
}
