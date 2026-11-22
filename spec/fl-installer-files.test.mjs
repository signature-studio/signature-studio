#!/usr/bin/env node
// spec/fl-installer-files.test.mjs : tools/fl-installer-files.mjs(インストーラの全ファイルの表)を確かめる。実物のインストーラは使わない。
// NSIS は合成する(ヘッダは LZMA1 で圧縮する。試験の中の小さな符号化器で作る。ファイルの中身は無圧縮)。PE は偽の物を作る。
// 一時ファイルはリポジトリの .tmp-spec-fi-* に作り、終わったら消す。終了コード 0 = 全部通った。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/fl-installer-files.mjs');
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const sha256 = (b) => crypto.createHash('sha256').update(b).digest('hex');

let m;
try {
  m = await import(pathToFileURL(TOOL).href);
} catch (e) {
  console.log(`FAIL import: ${e.message}`);
  process.exit(1);
}

// 偽の PE(32 ビットか 64 ビット)。import ディレクトリは RVA 0x1000、.idata の 1 節(raw 0x200 = RVA 0x1000)、名前は RVA 0x1100 から
function fakePe(dlls, { pe32 = false } = {}) {
  const b = Buffer.alloc(0x600);
  b.write('MZ', 0, 'latin1');
  b.writeUInt32LE(0x40, 0x3c);
  b.writeUInt32LE(0x4550, 0x40);
  b.writeUInt16LE(pe32 ? 0x14c : 0x8664, 0x44);
  b.writeUInt16LE(1, 0x46);
  const optSize = pe32 ? 224 : 240;
  b.writeUInt16LE(optSize, 0x54);
  const opt = 0x58;
  b.writeUInt16LE(pe32 ? 0x10b : 0x20b, opt);
  b.writeUInt32LE(0x200, opt + 60);
  b.writeUInt32LE(16, opt + (pe32 ? 92 : 108));
  const dirs = opt + (pe32 ? 96 : 112);
  b.writeUInt32LE(0x1000, dirs + 8);
  b.writeUInt32LE(0x100, dirs + 12);
  const s = opt + optSize;
  b.write('.idata', s, 'latin1');
  b.writeUInt32LE(0x100, s + 8);
  b.writeUInt32LE(0x1000, s + 12);
  b.writeUInt32LE(0x200, s + 16);
  b.writeUInt32LE(0x200, s + 20);
  dlls.forEach((name, i) => {
    b.writeUInt32LE(0x1100 + 0x40 * i, 0x200 + 20 * i + 12);
    b.writeUInt32LE(0x1200, 0x200 + 20 * i + 16);
    b.write(`${name}\0`, 0x300 + 0x40 * i, 'latin1');
  });
  return b;
}

// 試験用の符号化器: LZMA1 のリテラルだけ(lc=3 lp=0 pb=2)。終端の印は付けない(復号は長さを指定して読む)
function lzmaLiterals(data) {
  const isMatch = new Uint16Array(192).fill(1024);
  const lit = new Uint16Array(0x300 << 3).fill(1024);
  const out = [];
  let low = 0, range = 0xffffffff, cache = 0, cacheSize = 1;
  const shiftLow = () => {
    if (low < 0xff000000 || low >= 0x100000000) {
      const carry = low >= 0x100000000 ? 1 : 0;
      let temp = cache;
      do {
        out.push((temp + carry) & 0xff);
        temp = 0xff;
      } while (--cacheSize !== 0);
      cache = Math.floor(low / 0x1000000) & 0xff;
    }
    cacheSize++;
    low = (low % 0x1000000) * 256;
  };
  const bit = (probs, i, b) => {
    const p = probs[i];
    const bound = (range >>> 11) * p;
    if (b === 0) {
      range = bound;
      probs[i] = p + ((2048 - p) >>> 5);
    } else {
      low += bound;
      range -= bound;
      probs[i] = p - (p >>> 5);
    }
    while (range < 0x1000000) {
      range *= 256;
      shiftLow();
    }
  };
  let prev = 0;
  data.forEach((byte, pos) => {
    bit(isMatch, pos & 3, 0);
    const base = 0x300 * (prev >> 5);
    let sym = 1;
    for (let k = 7; k >= 0; k--) {
      const b = (byte >> k) & 1;
      bit(lit, base + sym, b);
      sym = (sym << 1) | b;
    }
    prev = byte;
  });
  for (let i = 0; i < 5; i++) shiftLow();
  return Buffer.from(out);
}

const PROPS = Buffer.from([0x5d, 0, 0, 1, 0]);
const MAGIC = Buffer.concat([Buffer.from([0xef, 0xbe, 0xad, 0xde]), Buffer.from('NullsoftInst')]);

// 合成の NSIS: files の順に、SetOutPath($INSTDIR\dir) と ExtractFile(name) を並べる。dir は '' か相対パス。name に $PLUGINSDIR を書けば、表から外れる
function nsisInstaller(files) {
  const units = [0];
  const str = (s) => {
    const at = units.length;
    s.split('$INSTDIR').forEach((piece, i) => {
      if (i > 0) units.push(3, 21);
      for (const ch of piece) units.push(ch.charCodeAt(0));
    });
    units.push(0);
    return at;
  };
  const entries = [];
  const streams = [];
  let streamOff = 0;
  for (const f of files) {
    entries.push([11, str(f.dir === '' ? '$INSTDIR' : `$INSTDIR\\${f.dir}`), 1, 0, 0, 0, 0]);
    entries.push([20, 1, str(f.name), streamOff, 0, 0, 0]);
    const w = Buffer.alloc(4);
    w.writeUInt32LE(f.data.length, 0);
    streams.push(Buffer.concat([w, f.data]));
    streamOff += 4 + f.data.length;
  }
  const N = entries.length, secOff = 68, entOff = secOff + 32, strOff = entOff + 28 * N;
  const strEnd = strOff + units.length * 2;
  const hdr = Buffer.alloc(strEnd);
  const blocks = [[0, 0], [secOff, 1], [entOff, N], [strOff, units.length], [strEnd, 0], [strEnd, 0], [strEnd, 0], [strEnd, 0]];
  blocks.forEach(([o, n], i) => {
    hdr.writeUInt32LE(o, 4 + 8 * i);
    hdr.writeUInt32LE(n, 8 + 8 * i);
  });
  hdr.writeInt32LE(0, secOff + 12);
  hdr.writeInt32LE(N, secOff + 16);
  entries.forEach((e, i) => e.forEach((v, k) => hdr.writeInt32LE(v, entOff + 28 * i + 4 * k)));
  units.forEach((u, i) => hdr.writeUInt16LE(u, strOff + 2 * i));
  const comp = lzmaLiterals(hdr);
  const fh = Buffer.alloc(28);
  MAGIC.copy(fh, 4);
  fh.writeUInt32LE(hdr.length, 20);
  const word = Buffer.alloc(4);
  word.writeUInt32LE((0x80000000 | (5 + comp.length)) >>> 0, 0);
  return Buffer.concat([Buffer.alloc(64, 0x4d), fh, word, PROPS, comp, ...streams]);
}

const FILES = [
  { dir: 'Plugins\\Fruity\\Effects', name: 'Fruity Balance_x64.dll', data: fakePe(['KERNEL32.dll', 'OpenGL32.dll']) },
  { dir: 'Plugins\\Fruity\\Generators', name: 'Foo.dll', data: fakePe(['kernel32.dll'], { pe32: true }) },
  { dir: 'Plugins\\Fruity\\Effects', name: 'Dup.dll', data: Buffer.from('old') },
  { dir: 'Plugins\\Fruity\\Effects', name: 'DUP.dll', data: fakePe(['user32.dll']) },
  { dir: 'Shared\\Artwork\\Fonts', name: 'a.ilfont', data: Buffer.from('font data') },
  { dir: 'Data\\Templates', name: 't.flp', data: Buffer.from('template') },
  { dir: '', name: 'Program.exe', data: fakePe(['msvcrt.dll']) },
  { dir: '', name: '$PLUGINSDIR\\tmp.dll', data: fakePe(['gdi32.dll']) },
];

// 1. 符号化器と復号の往復(試験の道具が正しいこと)
{
  const probe = Buffer.from('LZMA round trip, with some repeated text. '.repeat(20) + '\x00\xff');
  const { lzmaDecode } = await import(pathToFileURL(path.join(ROOT, 'app/scripts/lib/lzma1.mjs')).href);
  ok('符号化器: リテラルだけの LZMA1 を、既存の復号が元に戻す', Buffer.compare(lzmaDecode(PROPS, lzmaLiterals(probe), probe.length), probe) === 0, true);
}

// 2. 偽の PE の import(32 ビットと 64 ビット)と、PE でない物
ok('peImports: 64 ビットの PE の import を、小文字で、順に取る', m.peImports(fakePe(['KERNEL32.dll', 'OpenGL32.dll'])), ['kernel32.dll', 'opengl32.dll']);
ok('peImports: 32 ビットの PE も取る', m.peImports(fakePe(['User32.dll'], { pe32: true })), ['user32.dll']);
ok('peImports: PE でない物は空', m.peImports(Buffer.from('plain text, not a PE image, long enough to pass the length check')), []);
ok('kindOf: 拡張子で dll・exe・other(大文字の拡張子も dll)', ['X.DLL', 'a.exe', 'b.flp'].map(m.kindOf), ['dll', 'exe', 'other']);
ok('hashed: 他の場所の other は計算しない。Plugins と Data の下は計算する', [
  m.hashed('Shared\\x.ilfont', 'other'), m.hashed('Data\\t.flp', 'other'), m.hashed('Plugins\\x.txt', 'other'), m.hashed('Shared\\y.dll', 'dll'),
], [false, true, true, true]);

// 3. 合成の NSIS を、表の道具で読む(CLI)
const T = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-fi-'));
try {
  const inst = path.join(T, 'installer.exe');
  const out = path.join(T, 'out', 'fl-installer-files.tsv');
  fs.writeFileSync(inst, nsisInstaller(FILES));
  const run = spawnSync(process.execPath, [TOOL, inst, out], { encoding: 'utf8' });
  ok('CLI: 合成のインストーラから表を書き、終了コード 0', run.status, 0);
  const text = fs.existsSync(out) ? fs.readFileSync(out, 'utf8') : '';
  const lines = text.split('\n').filter((l) => l !== '');
  ok('表: 1 行目は方針の注釈(# で始まる)、2 行目は見出し', [lines[0].startsWith('#'), lines[1]], [true, 'guest\tbytes\tsha256\tkind\timports']);
  const rows = lines.slice(2).map((l) => l.split('\t'));
  ok('表: 表に出るのは、$PLUGINSDIR の物を除く 6 件', rows.length, 6);
  const dllB = FILES[3].data, exeB = FILES[6].data;
  const want = [
    ['Data\\Templates\\t.flp', String(FILES[5].data.length), sha256(FILES[5].data), 'other', ''],
    ['Plugins\\Fruity\\Effects\\DUP.dll', String(dllB.length), sha256(dllB), 'dll', 'user32.dll'],
    ['Plugins\\Fruity\\Effects\\Fruity Balance_x64.dll', String(FILES[0].data.length), sha256(FILES[0].data), 'dll', 'kernel32.dll;opengl32.dll'],
    ['Plugins\\Fruity\\Generators\\Foo.dll', String(FILES[1].data.length), sha256(FILES[1].data), 'dll', 'kernel32.dll'],
    ['Program.exe', String(exeB.length), sha256(exeB), 'exe', 'msvcrt.dll'],
    ['Shared\\Artwork\\Fonts\\a.ilfont', '', '', 'other', ''],
  ];
  ok('表: 行は guest の小文字の順。同じ guest は後の物。接頭辞は外れている', rows, want);
  ok('表: 会社名を含むパスも、ドライブ名も、インストール先の接頭辞も出ない', [/[A-Za-z]:\\/.test(text), /(^|\n)install\\/.test(text), /Image-Line|Program Files/.test(text)], [false, false, false]);
  const cli = spawnSync(process.execPath, [TOOL], { encoding: 'utf8' });
  ok('CLI: 引数が無ければ使い方を出し、終了コード 64', cli.status, 64);
  const bad = spawnSync(process.execPath, [TOOL, path.join(T, 'missing.exe'), out], { encoding: 'utf8' });
  ok('CLI: インストーラが無ければ 0 以外で終わる', bad.status !== 0, true);
} finally {
  fs.rmSync(T, { recursive: true, force: true });
}

// 4. buildInventory(依存の注入): 読むのは計算する行だけ。同じ guest は後の物。入力の順に依らない
{
  const bufs = new Map([[1, fakePe(['KERNEL32.dll', 'OpenGL32.dll'])], [2, Buffer.from('dup-old')], [3, Buffer.from('dup-new')], [4, Buffer.from('font')], [5, Buffer.from('template')]]);
  const calls = [];
  const read = (off) => {
    calls.push(off);
    return bufs.get(off);
  };
  const entries = [
    ['Plugins\\Fruity\\Effects\\Fruity Balance_x64.dll', 1],
    ['Plugins\\Fruity\\Effects\\dup.dll', 2],
    ['Plugins\\Fruity\\Effects\\DUP.dll', 3],
    ['Shared\\Artwork\\Fonts\\a.ilfont', 4],
    ['Data\\Templates\\t.flp', 5],
  ];
  const rows = m.buildInventory({ files: entries, read });
  ok('buildInventory: 読むのは計算する行だけ(Shared の other は読まない)', calls.sort(), [1, 3, 5]);
  ok('buildInventory: 同じ guest は後の物(大きさと SHA-256 も)', [rows[1].guest, rows[1].bytes, rows[1].sha256 === sha256(bufs.get(3))], ['Plugins\\Fruity\\Effects\\DUP.dll', String(bufs.get(3).length), true]);
  const single = entries.filter(([, off]) => off !== 2);
  const again = m.buildInventory({ files: [...single].reverse(), read });
  ok('buildInventory: 重複の無い入力は、順を変えても同じ表になる', JSON.stringify(m.buildInventory({ files: single, read })) === JSON.stringify(again), true);
  ok('buildInventory: タブを含む guest は受け付けない', (() => {
    try {
      m.buildInventory({ files: [['a\tb', 1]], read });
      return 'accepted';
    } catch {
      return 'threw';
    }
  })(), 'threw');
  const text = m.tsv(rows);
  ok('tsv: 注釈・見出し・行。見出しは guest の列から始まる', [text.split('\n')[0].startsWith('#'), text.split('\n')[1], text.trimEnd().split('\n').length], [true, 'guest\tbytes\tsha256\tkind\timports', 2 + rows.length]);
  ok('tsv: 各行の列の数は 5(空の列も数える)', text.split('\n').slice(2, -1).map((l) => l.split('\t').length), rows.map(() => 5));
}

process.exitCode = failed ? 1 : 0;
