// tools/plugin-classes/pe.mjs : PE の RVA からファイルの位置への対応と、export の名前の読み。節の対応は tools/fl-installer-files.mjs の peImports と同じ。
export function peLayout(d) {
  if (d.length < 0x40 || d.toString('latin1', 0, 2) !== 'MZ') return null;
  const nt = d.readUInt32LE(0x3c);
  if (nt + 24 > d.length || d.readUInt32LE(nt) !== 0x4550) return null;
  const opt = nt + 24;
  const magic = d.readUInt16LE(opt);
  if (magic !== 0x10b && magic !== 0x20b) return null;
  const is64 = magic === 0x20b;
  const dirs = opt + (is64 ? 112 : 96);
  const nSec = d.readUInt16LE(nt + 6), optSize = d.readUInt16LE(nt + 20), hdrSize = d.readUInt32LE(opt + 60);
  const secs = [];
  for (let i = 0; i < nSec; i++) {
    const s = opt + optSize + 40 * i;
    secs.push({ va: d.readUInt32LE(s + 12), raw: d.readUInt32LE(s + 16), ptr: d.readUInt32LE(s + 20) });
  }
  const off = (rva) => {
    if (rva < hdrSize) return rva;
    const s = secs.find((x) => rva >= x.va && rva - x.va < x.raw);
    return s ? s.ptr + rva - s.va : -1;
  };
  const hasExportDir = d.readUInt32LE(opt + (is64 ? 108 : 92)) >= 1;
  return { off, exportRva: hasExportDir ? d.readUInt32LE(dirs) : 0 };
}

// The bytes at a RVA (n bytes), or null when the RVA is not in the file.
export function bytesAtRva(d, rva, n) {
  const L = peLayout(d);
  if (!L) return null;
  const at = L.off(rva);
  if (at < 0 || at + n > d.length) return null;
  return d.subarray(at, at + n);
}

// The named exports of a PE image, in the order of its name table. [] when the bytes are not a PE image or have no export directory.
export function exportNames(d) {
  const L = peLayout(d);
  if (!L || L.exportRva === 0) return [];
  const at = L.off(L.exportRva);
  if (at < 0 || at + 40 > d.length) return [];
  const count = d.readUInt32LE(at + 24);
  const namesAt = L.off(d.readUInt32LE(at + 32));
  if (namesAt < 0) return [];
  const names = [];
  for (let i = 0; i < count && namesAt + 4 * i + 4 <= d.length; i++) {
    const p = L.off(d.readUInt32LE(namesAt + 4 * i));
    if (p < 0 || p >= d.length) break;
    const end = d.indexOf(0, p);
    if (end < 0) break;
    names.push(d.toString('latin1', p, end));
  }
  return names;
}
