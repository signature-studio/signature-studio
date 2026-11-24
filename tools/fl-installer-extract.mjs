#!/usr/bin/env node
// tools/fl-installer-extract.mjs INSTALLER OUT_DIR [--file GUEST]... [ENTRY...] : FL Studio のインストーラ(NSIS)が入れるファイルのうち、ENTRY に一致する物を、OUT_DIR の下に、インストール先からの相対のパス(\ を / にして)で書く。
// ENTRY は、行の形で分ける: \ で終わる物は接頭辞(その下の全ファイル)、\ で終わらない物は完全なパス(--file GUEST と同じ)。ci/resources-prefixes.txt の 1 行は、どちらか 1 つ。
// 読みは tools/fl-installer-files.mjs と同じ(app/scripts/lib の nsis.mjs と nsis-script.mjs を読むだけ。app/ は変えない)。
// 接頭辞と完全なパスは、大文字・小文字を区別しない。完全なパスは、インストール先からの完全なパス(根の直下の FL64.exe など)。一致する物が無い接頭辞は標準エラーに警告を出して続ける。一覧に無い完全なパスが 1 つでもあれば、1 で終わり、何も書かない。一致が全く無ければ 1 で終わる。
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
    else if (rest[i].endsWith('\\')) prefixes.push(rest[i]);
    else guests.push(rest[i]);
  }
  if (!installer || !outDir || (prefixes.length === 0 && guests.length === 0) || !prefixes.every((p) => p.length > 1) || !guests.every((g) => g && !g.endsWith('\\'))) {
    console.log('usage: tools/fl-installer-extract.mjs INSTALLER OUT_DIR [--file GUEST]... [ENTRY...]   (\\ で終わる ENTRY は接頭辞。それ以外は完全なパスで、\\ で終わらない)');
    return 64;
  }
  const inst = new Installer(installer);
  try {
    const script = new Script(inst);
    const files = installTree(script, ROOT).map(([, p, off]) => [p.slice(ROOT.length + 1), off]);
    const exact = selectExact(files, guests);
    const missing = guests.filter((g) => !exact.some((r) => r.guest.toLowerCase() === g.toLowerCase()));
    if (missing.length > 0) {
      for (const g of missing) console.error(`fl-installer-extract: エラー: 一覧に無い完全なパス ${g}`);
      return 1;
    }
    const picked = new Map();
    for (const r of [...selectFiles(files, prefixes), ...exact]) picked.set(r.guest.toLowerCase(), r);
    const rows = [...picked.keys()].sort().map((key) => picked.get(key));
    for (const p of prefixes) {
      if (!rows.some((r) => r.guest.toLowerCase().startsWith(p.toLowerCase()))) console.error(`fl-installer-extract: 警告: 接頭辞 ${p} に一致する物が無い`);
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
