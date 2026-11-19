// Prepares a conformance run of lift/harness/seeds/vfs_lazy_conformance.c (decision 0314): from an index directory made by tools/vfs-build.py (index.bin + vfs/<sha256>.chk) it writes
// the stream files <out>/vfs/<stream name> (the decoded chunks concatenated; every chunk is checked against its name), <out>/vfs/index.bin, the seed list <out>/seed.tsv (the "@lazy:" row) and <out>/paths.tsv (guest path TAB original file).
// usage: node lazy-conformance-prep.mjs <index dir> <out dir> <original root> [sample N | 0 = every file]   (node 24: the TypeScript of app/src/engine is read directly)
import { brotliDecompressSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { closeSync, copyFileSync, existsSync, mkdirSync, openSync, readFileSync, writeFileSync, writeSync } from 'node:fs';
import { join } from 'node:path';
import { parseIndex } from '../../../app/src/engine/vfs-index.ts';

const [, , src, out, original, sampleArg = '0'] = process.argv;
if (!src || !out || !original) { console.error('usage: node lazy-conformance-prep.mjs <index dir> <out dir> <original root> [sample N]'); process.exit(64); }
const bin = readFileSync(join(src, 'index.bin'));
const idx = parseIndex(new Uint8Array(bin), { entries: true });
mkdirSync(join(out, 'vfs'), { recursive: true });
copyFileSync(join(src, 'index.bin'), join(out, 'vfs', 'index.bin'));
let wrote = 0;
for (const s of idx.streams) {
  const fd = openSync(join(out, 'vfs', s.name), 'w');
  for (const sha of s.chunks) {
    const stored = readFileSync(join(src, 'vfs', `${sha}.chk`));
    const b = s.coding === 1 ? brotliDecompressSync(stored) : stored;
    if (createHash('sha256').update(b).digest('hex') !== sha) throw new Error(`chunk ${sha}: the decoded bytes do not hash to the name`);
    writeSync(fd, b);
    wrote += b.length;
  }
  closeSync(fd);
}
const mount = idx.mount;
writeFileSync(join(out, 'seed.tsv'), `${mount}\\\t@lazy:vfs/index.bin\t${createHash('sha256').update(bin).digest('hex')}\n`);
const files = idx.entries.filter((e) => e.kind === 'file');
const pick = new Set();
const n = Number(sampleArg);
if (n === 0) files.forEach((_, i) => pick.add(i));
else {
  for (let k = 0; k < files.length; k += Math.max(1, Math.floor(files.length / n))) pick.add(k);
  files.map((f, i) => [f.size, i]).sort((a, b) => b[0] - a[0]).slice(0, 20).forEach(([, i]) => pick.add(i));
  files.forEach((f, i) => { if (f.size === 0 && pick.size < n + 40) pick.add(i); });
}
const lines = [];
for (const i of [...pick].sort((a, b) => a - b)) {
  const e = files[i];
  const p = join(original, e.path);
  if (existsSync(p)) lines.push(`${mount}\\${e.path.split('/').join('\\')}\t${p}`);
}
writeFileSync(join(out, 'paths.tsv'), `${lines.join('\n')}\n`);
console.log(`prep: ${idx.nFiles} files + ${idx.nDirs} dirs in the index, ${idx.streams.length} streams, ${wrote} stream bytes written, ${lines.length} paths in paths.tsv`);
