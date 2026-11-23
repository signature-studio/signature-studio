#!/usr/bin/env node
// tools/fl-installer-extract.mjs INSTALLER OUT_DIR [--file GUEST]... [PREFIX...] : FL Studio のインストーラ(NSIS)が入れるファイルのうち、接頭辞に一致する全ファイルと、--file の完全なパスの物を、OUT_DIR の下に、インストール先からの相対のパス(\ を / にして)で書く。
// 読みは tools/fl-installer-files.mjs と同じ(app/scripts/lib の nsis.mjs と nsis-script.mjs を読むだけ。app/ は変えない)。
// 接頭辞は \ で終わり、大文字・小文字を区別しない。--file GUEST は、インストール先からの完全なパス(根の直下の FL64.exe など。\ で終わらない)で、大文字・小文字を区別しない。一致する物が無い接頭辞・完全なパスは標準エラーに警告を出して続ける。一致が全く無ければ 1 で終わる。
// 同じ guest パスが 2 回出たら、後の物を使う(extract-fl.mjs と同じ)。OUT_DIR/MANIFEST.tsv に、guest・bytes・sha256 の表を書く(guest の小文字の順)。
// 会社名を含む絶対パスは、どちらにも書かない(インストール先の接頭辞を外す)。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Installer, streamFile } from '../app/scripts/lib/nsis.mjs';
import { Script, installTree } from '../app/scripts/lib/nsis-script.mjs';

const ROOT = 'install';
const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');

function checkGuest(guest) {
  if (/[\t\r\n]/.test(guest)) throw new Error(`fl-installer-extract: 改行かタブを含むパス: ${JSON.stringify(guest)}`);
  if (guest.split('\\').some((s) => s === '' || s === '.' || s === '..' || s.includes('/') || s.includes(':'))) {
    throw new Error(`fl-installer-extract: 安全でないパス: ${JSON.stringify(guest)}`);
  }
}

// files: [guest, stream offset][]. Returns { guest, off } of the files whose guest path starts with one of the prefixes (case-insensitive), sorted by the lower-case guest path.
export function selectFiles(files, prefixes) {
  const last = new Map();
  for (const [guest, off] of files) last.set(guest.toLowerCase(), { guest, off });
  const lower = prefixes.map((p) => p.toLowerCase());
  return [...last.keys()].sort()
    .filter((key) => lower.some((p) => key.startsWith(p)))
    .map((key) => {
      const row = last.get(key);
      checkGuest(row.guest);
      return row;
    });
}

// files: [guest, stream offset][]. Returns { guest, off } of the files whose guest path equals one of the given full paths (case-insensitive), sorted by the lower-case guest path.
export function selectExact(files, guests) {
  const last = new Map();
  for (const [guest, off] of files) last.set(guest.toLowerCase(), { guest, off });
  const want = new Set(guests.map((g) => g.toLowerCase()));
  return [...last.keys()].sort()
    .filter((key) => want.has(key))
    .map((key) => {
      const row = last.get(key);
      checkGuest(row.guest);
      return row;
    });
}

function place(dest, data) {
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, data);
}

function main(argv) {
  const [installer, outDir, ...rest] = argv;
  const prefixes = [];
  const guests = [];
  for (let i = 0; i < rest.length; i++) {
    if (rest[i] === '--file') guests.push(rest[++i]);
    else prefixes.push(rest[i]);
  }
  if (!installer || !outDir || (prefixes.length === 0 && guests.length === 0) || !prefixes.every((p) => p.length > 1 && p.endsWith('\\')) || !guests.every((g) => g && !g.endsWith('\\'))) {
    console.log('usage: tools/fl-installer-extract.mjs INSTALLER OUT_DIR [--file GUEST]... [PREFIX...]   (PREFIX は \\ で終わる。GUEST は完全なパスで、\\ で終わらない)');
    return 64;
  }
  const inst = new Installer(installer);
  try {
    const script = new Script(inst);
    const files = installTree(script, ROOT).map(([, p, off]) => [p.slice(ROOT.length + 1), off]);
    const picked = new Map();
    for (const r of [...selectFiles(files, prefixes), ...selectExact(files, guests)]) picked.set(r.guest.toLowerCase(), r);
    const rows = [...picked.keys()].sort().map((key) => picked.get(key));
    for (const p of prefixes) {
      if (!rows.some((r) => r.guest.toLowerCase().startsWith(p.toLowerCase()))) console.error(`fl-installer-extract: 警告: 接頭辞 ${p} に一致する物が無い`);
    }
    for (const g of guests) {
      if (!rows.some((r) => r.guest.toLowerCase() === g.toLowerCase())) console.error(`fl-installer-extract: 警告: 完全なパス ${g} に一致する物が無い`);
    }
    if (rows.length === 0) {
      console.error('fl-installer-extract: 一致が 1 件も無い');
      return 1;
    }
    const manifest = ['guest\tbytes\tsha256\n'];
    for (const { guest, off } of rows) {
      const data = streamFile(inst, script.info.dataStart, off);
      place(path.join(outDir, ...guest.split('\\')), data);
      manifest.push(`${guest}\t${data.length}\t${sha256(data)}\n`);
    }
    fs.writeFileSync(path.join(outDir, 'MANIFEST.tsv'), manifest.join(''));
    console.log(`fl-installer-extract: ${rows.length} 件を書いた(接頭辞 ${prefixes.length} 件)`);
    return 0;
  } finally {
    inst.close();
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
