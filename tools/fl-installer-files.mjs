#!/usr/bin/env node
// tools/fl-installer-files.mjs INSTALLER OUT.tsv : FL Studio のインストーラ(NSIS)が入れる、インストール先の全ファイルの表を作る(台帳 L-033。pm-work/issue-drafts/013-*.md)。
// 読みは app/scripts/extract-fl.mjs と同じ(lib/nsis.mjs と lib/nsis-script.mjs)。インストーラは読むだけで、app/ は変えない。
// 表(TAB 区切り): 先頭は # で始まる方針の注釈、次が見出し、残りが行。列は guest(インストール先からの相対パス)・bytes(展開後の大きさ)・sha256(展開後)・kind(dll・exe・other)・imports。
// bytes と sha256 は、dll・exe と、Plugins\ と Data\ の下の物だけ計算する。他の行は空にする。imports は dll と exe の PE が import する DLL の名前(小文字、; 区切り)。
// インストール先の既定の値は会社名を含む絶対パスなので、表には書かない。installTree には中立の接頭辞(install)を渡し、表では接頭辞を外す。
// 同じ guest パスが 2 回出たら、後の物を使う(extract-fl.mjs の Map と同じ)。行は guest パスの小文字の順。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Installer, streamFile } from '../app/scripts/lib/nsis.mjs';
import { Script, installTree } from '../app/scripts/lib/nsis-script.mjs';

const ROOT = 'install';
const NOTE = '# インストール先の全ファイル(接頭辞を外した相対パス)。bytes と sha256 は、dll・exe と Plugins\\ と Data\\ の下の物だけ(他は空)。imports は dll と exe が import する DLL の名前(小文字、; 区切り)';
const HEADER = ['guest', 'bytes', 'sha256', 'kind', 'imports'];
const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');

export function kindOf(guest) {
  const g = guest.toLowerCase();
  if (g.endsWith('.dll')) return 'dll';
  if (g.endsWith('.exe')) return 'exe';
  return 'other';
}

export function hashed(guest, kind) {
  return kind !== 'other' || /^(plugins|data)\\/i.test(guest);
}

// The DLL names that a PE image imports (its import directory), lower case, in the order of the image. [] when the bytes are not a PE image.
export function peImports(d) {
  if (d.length < 0x40 || d.toString('latin1', 0, 2) !== 'MZ') return [];
  const nt = d.readUInt32LE(0x3c);
  if (nt + 24 > d.length || d.readUInt32LE(nt) !== 0x4550) return [];
  const opt = nt + 24;
  const magic = d.readUInt16LE(opt);
  if (magic !== 0x10b && magic !== 0x20b) return [];
  const is64 = magic === 0x20b;
  const dirs = opt + (is64 ? 112 : 96);
  const dirRva = d.readUInt32LE(dirs + 8);
  if (d.readUInt32LE(opt + (is64 ? 108 : 92)) < 2 || dirRva === 0) return [];
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
  const names = [];
  for (let i = 0; i < 1024; i++) {
    const at = off(dirRva + 20 * i);
    if (at < 0 || at + 20 > d.length) break;
    if (d.subarray(at, at + 20).every((b) => b === 0)) break;
    const nameAt = off(d.readUInt32LE(at + 12));
    if (nameAt < 0 || nameAt >= d.length) break;
    const end = d.indexOf(0, nameAt);
    if (end < 0) break;
    const name = d.toString('latin1', nameAt, end).toLowerCase();
    if (!names.includes(name)) names.push(name);
  }
  return names;
}

// files: [guest, stream offset][]. read(offset) returns the bytes of that file, and is called only for the rows that hashed() names.
export function buildInventory({ files, read }) {
  const last = new Map();
  for (const [guest, off] of files) {
    if (/[\t\r\n]/.test(guest)) throw new Error(`inventory: 改行かタブを含むパス: ${JSON.stringify(guest)}`);
    last.set(guest.toLowerCase(), { guest, off });
  }
  return [...last.keys()].sort().map((key) => {
    const { guest, off } = last.get(key);
    const kind = kindOf(guest);
    const row = { guest, bytes: '', sha256: '', kind, imports: '' };
    if (!hashed(guest, kind)) return row;
    const data = read(off);
    row.bytes = String(data.length);
    row.sha256 = sha256(data);
    if (kind !== 'other') row.imports = peImports(data).join(';');
    return row;
  });
}

export function tsv(rows) {
  return [NOTE, HEADER, ...rows.map((r) => [r.guest, r.bytes, r.sha256, r.kind, r.imports])]
    .map((cols) => `${Array.isArray(cols) ? cols.join('\t') : cols}\n`)
    .join('');
}

function main(argv) {
  const [installer, out] = argv;
  if (!installer || !out) {
    console.log('usage: tools/fl-installer-files.mjs INSTALLER OUT.tsv');
    return 64;
  }
  const inst = new Installer(installer);
  try {
    const script = new Script(inst);
    const files = installTree(script, ROOT).map(([, p, off]) => [p.slice(ROOT.length + 1), off]);
    const rows = buildInventory({ files, read: (off) => streamFile(inst, script.info.dataStart, off) });
    fs.mkdirSync(path.dirname(out), { recursive: true });
    fs.writeFileSync(out, tsv(rows));
    console.log(`fl-installer-files: ${rows.length} 件を書いた(sha256 がある物は ${rows.filter((r) => r.sha256).length} 件)`);
    return 0;
  } finally {
    inst.close();
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
