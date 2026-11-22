// The facts of a PE image that the external-plugin list needs: whether a file is a PE image, and the DLLs its import table names.

export function isPe(buf) {
  if (buf.length < 0x40 || buf.readUInt16LE(0) !== 0x5a4d) return false;
  const e = buf.readUInt32LE(0x3c);
  return e + 4 <= buf.length && buf.toString('latin1', e, e + 4) === 'PE\0\0';
}

/** the names of the DLLs that the PE image `buf` imports, in lower case, in the order of its import table */
export function peImports(buf) {
  if (!isPe(buf)) throw new Error('not a PE image');
  const coff = buf.readUInt32LE(0x3c) + 4;
  const sectionCount = buf.readUInt16LE(coff + 2);
  const optSize = buf.readUInt16LE(coff + 16);
  const opt = coff + 20;
  const magic = buf.readUInt16LE(opt);
  if (magic !== 0x10b && magic !== 0x20b) throw new Error('unknown optional header');
  const importVA = buf.readUInt32LE(opt + (magic === 0x20b ? 112 : 96) + 8);
  const sections = [];
  for (let i = 0; i < sectionCount; i++) {
    const s = opt + optSize + 40 * i;
    sections.push({ va: buf.readUInt32LE(s + 12), size: Math.max(buf.readUInt32LE(s + 8), buf.readUInt32LE(s + 16)), ptr: buf.readUInt32LE(s + 20) });
  }
  const offsetOf = (rva) => {
    const s = sections.find((x) => rva >= x.va && rva < x.va + x.size);
    if (!s) throw new Error(`PE: RVA ${rva} is in no section`);
    return s.ptr + rva - s.va;
  };
  if (importVA === 0) return [];
  const names = [];
  for (let d = offsetOf(importVA); ; d += 20) {
    const nameRVA = buf.readUInt32LE(d + 12);
    if (nameRVA === 0) break;
    const p = offsetOf(nameRVA);
    names.push(buf.toString('latin1', p, buf.indexOf(0, p)).toLowerCase());
  }
  return names;
}
