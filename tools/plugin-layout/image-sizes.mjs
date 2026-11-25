#!/usr/bin/env node
// Issue #1379: the SizeOfImage of each plugin DLL of the installer, the input of the layout test of fe-build (plugins.rs). Usage:
//   node tools/plugin-layout/image-sizes.mjs > inputs/plugin-image-sizes.tsv
// Input: inputs/plugin-classes.tsv (the installer's DLLs: path, size, SHA-256) and the installer's tree lift/out-app/host (extracted by
// app/scripts/extract-fl.mjs; not committed). The DLLs of the tree are matched by SHA-256 with the rows; a row without a DLL of the tree is left out
// (the VFX folders are not extracted; the layout test reads only the plan's folders and says so when a row it needs is missing).
// Output (stdout): one row per matched DLL: SHA-256, file size, SizeOfImage (the PE optional header), and the installer path.
// Nothing is written except stdout. Exit code 0 on success, 1 when a matched file is not a PE, 2 on a usage error.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

if (process.argv.length !== 2) {
  process.stderr.write('usage: node tools/plugin-layout/image-sizes.mjs > inputs/plugin-image-sizes.tsv\n');
  process.exit(2);
}
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const host = join(root, 'lift', 'out-app', 'host');
const rows = fs.readFileSync(join(root, 'inputs', 'plugin-classes.tsv'), 'utf8').replace(/\r\n/g, '\n').split('\n').slice(1).filter((l) => l !== '').map((l) => {
  const f = l.split('\t');
  return { guest: f[0], size: Number(f[1]), sha: f[2] };
});
const bySize = new Map();
for (const r of rows) {
  if (!bySize.has(r.size)) bySize.set(r.size, []);
  bySize.get(r.size).push(r);
}

/** the SizeOfImage of a PE32 or PE32+ file (the optional header's SizeOfImage is at +56 in both), or an error text */
function sizeOfImage(data) {
  if (data.length < 0x40 || data.readUInt16LE(0) !== 0x5a4d) return 'not a PE file (no MZ)';
  const e = data.readUInt32LE(0x3c);
  if (data.toString('latin1', e, e + 4) !== 'PE\0\0') return 'not a PE file (no PE signature)';
  const opt = e + 24;
  if (data.readUInt16LE(opt) !== 0x20b && data.readUInt16LE(opt) !== 0x10b) return 'not a PE file (no optional header magic)';
  return data.readUInt32LE(opt + 56);
}

function* files(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) yield* files(p);
    else if (e.isFile() && e.name.toLowerCase().endsWith('.dll')) yield p;
  }
}

const found = new Map();
for (const p of files(host)) {
  const size = fs.statSync(p).size;
  const cands = bySize.get(size);
  if (!cands) continue;
  const data = fs.readFileSync(p);
  const sha = crypto.createHash('sha256').update(data).digest('hex');
  const hit = cands.find((r) => r.sha === sha);
  if (!hit || found.has(sha)) continue;
  const image = sizeOfImage(data);
  if (typeof image !== 'number') {
    process.stderr.write(`image-sizes: ${hit.guest}: ${image}\n`);
    process.exit(1);
  }
  found.set(sha, { size, image, guest: hit.guest });
}

const missing = rows.filter((r) => !found.has(r.sha));
for (const r of missing) process.stderr.write(`image-sizes: left out (no DLL of the tree): ${r.guest}\n`);
process.stdout.write('sha256\tsize\tSizeOfImage\tguest\n');
for (const r of rows) {
  const f = found.get(r.sha);
  if (!f) continue;
  process.stdout.write(`${r.sha}\t${f.size}\t${f.image}\t${r.guest}\n`);
}
