#!/usr/bin/env node
// spec/fl-installer-extract.test.mjs : tools/fl-installer-extract.mjs(接頭辞に一致する全ファイルを、インストール先の相対のパスで書く)を確かめる。実物のインストーラは使わない。
// NSIS は合成する(ヘッダは LZMA1 で圧縮する。試験の中の小さな符号化器で作る。ファイルの中身は無圧縮)。spec/fl-installer-files.test.mjs の合成と同じ作り。
// 一時ファイルはリポジトリの .tmp-spec-fe-* に作り、終わったら消す。終了コード 0 = 全部通った。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/fl-installer-extract.mjs');
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
  { dir: 'Plugins\\Fruity\\Effects', name: 'Fruity Balance_x64.dll', data: Buffer.from('balance dll') },
  { dir: 'Plugins\\Fruity\\Effects', name: 'Readme.txt', data: Buffer.from('a file of the prefix, not a DLL') },
  { dir: 'Plugins\\Fruity\\Effects', name: 'Dup.dll', data: Buffer.from('old') },
  { dir: 'Plugins\\Fruity\\Effects', name: 'DUP.dll', data: Buffer.from('new') },
  { dir: 'Plugins\\Fruity\\EffectsX', name: 'Other.dll', data: Buffer.from('outside, a longer name') },
  { dir: 'Plugins\\Fruity\\Generators\\FLEX', name: 'engine_x64.dll', data: Buffer.from('flex') },
  { dir: 'Artwork\\Themes', name: 'Ash.flstheme', data: Buffer.from('theme') },
  { dir: 'Shared\\Artwork', name: 'x.png', data: Buffer.from('shared, not the prefix') },
  { dir: 'Data\\System', name: 'Empty.flp', data: Buffer.from('project') },
  { dir: 'Data\\System', name: 'Sub\\deep.txt', data: Buffer.from('deep') },
  { dir: '', name: 'Program.exe', data: Buffer.from('MZ outside') },
  { dir: '', name: '$PLUGINSDIR\\tmp.dll', data: Buffer.from('tmp') },
];

// 1. selectFiles(依存の注入): 接頭辞は段の単位で一致し、大文字・小文字を区別せず、同じ guest は後の物、順は guest の小文字の順
{
  const rows = m.selectFiles([['Artwork\\Themes\\a.flstheme', 1], ['artwork\\themes\\A.flstheme', 2], ['Data\\System.nfo', 3], ['Data\\SystemX\\y', 4], ['Data\\System\\z.flp', 5]], ['Data\\System\\']);
  ok('selectFiles: 接頭辞は段の単位で一致する(Data\\SystemX と Data\\System.nfo は外れる)', rows, [{ guest: 'Data\\System\\z.flp', off: 5 }]);
  ok('selectFiles: 同じ guest(大文字・小文字の違い)は後の物', m.selectFiles([['Artwork\\A.png', 1], ['artwork\\a.PNG', 2]], ['artwork\\']), [{ guest: 'artwork\\a.PNG', off: 2 }]);
  ok('selectFiles: 接頭辞の大文字・小文字を区別しない', m.selectFiles([['Plugins\\Fruity\\Effects\\a.dll', 1]], ['PLUGINS\\fruity\\EFFECTS\\']).length, 1);
  ok('selectFiles: 一致しない接頭辞には何も選ばない', m.selectFiles([['Artwork\\a', 1]], ['Data\\']), []);
  ok('selectFiles: 選んだ物の順は guest の小文字の順', m.selectFiles([['Artwork\\b', 1], ['Artwork\\A', 2]], ['Artwork\\']).map((r) => r.guest), ['Artwork\\A', 'Artwork\\b']);
  const verdict = (list, pre) => {
    try {
      m.selectFiles(list, pre);
      return 'accepted';
    } catch {
      return 'threw';
    }
  };
  ok('selectFiles: 選んだ物に .. の段があれば落ちる', verdict([['Artwork\\..\\x', 1]], ['Artwork\\']), 'threw');
  ok('selectFiles: コロンを含む段(絶対パス)は落ちる', verdict([['Artwork\\C:x', 1]], ['Artwork\\']), 'threw');
  ok('selectFiles: 選んでいない物の安全でないパスでは落ちない', verdict([['..\\y', 1], ['Artwork\\a', 2]], ['Artwork\\']), 'accepted');
  const exactList = [['FL64.exe', 1], ['FL64 (scaled).exe', 2], ['FL64.exe.bak', 3], ['Artwork\\a', 4]];
  ok('selectExact: 完全なパスだけを、大文字・小文字を区別せず、guest の小文字の順に選ぶ', m.selectExact(exactList, ['fl64.EXE', 'FL64 (scaled).exe']), [{ guest: 'FL64 (scaled).exe', off: 2 }, { guest: 'FL64.exe', off: 1 }]);
  ok('selectExact: 接頭辞の一致では選ばない(FL64 では何も選ばない)', m.selectExact(exactList, ['FL64']), []);
  ok('selectExact: 同じ guest は後の物', m.selectExact([['FL64.exe', 1], ['fl64.exe', 2]], ['FL64.exe']), [{ guest: 'fl64.exe', off: 2 }]);
  ok('selectExact: 指定の無い物は選ばない', m.selectExact(exactList, []), []);
}

// 2. 合成のインストーラを、道具で読む(CLI)
const T = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-fe-'));
try {
  const inst = path.join(T, 'installer.exe');
  fs.writeFileSync(inst, nsisInstaller(FILES));
  const out = path.join(T, 'out');
  const run = spawnSync(process.execPath, [TOOL, inst, out, 'Plugins\\Fruity\\Effects\\', 'artwork\\', 'Data\\Projects\\', 'Data\\System\\'], { encoding: 'utf8' });
  ok('CLI: 一致する接頭辞があれば終了コード 0', run.status, 0);
  ok('CLI: 一致しない接頭辞(Data\\Projects\\)だけに、警告を 1 件出す', run.stderr.split('\n').filter((l) => l.includes('警告')).map((l) => l.includes('Data\\Projects\\')), [true]);
  const listed = fs.readdirSync(out, { recursive: true }).filter((f) => fs.statSync(path.join(out, f)).isFile()).sort();
  const wantFiles = ['Artwork/Themes/Ash.flstheme', 'Data/System/Empty.flp', 'Data/System/Sub/deep.txt', 'MANIFEST.tsv', 'Plugins/Fruity/Effects/DUP.dll', 'Plugins/Fruity/Effects/Fruity Balance_x64.dll', 'Plugins/Fruity/Effects/Readme.txt'].sort();
  ok('CLI: 書かれた物は、接頭辞に一致する全ファイル(同じ guest は後の物 1 つ)と MANIFEST だけ', listed, wantFiles);
  ok('CLI: 中身はインストーラの物と同じ(接頭辞の下の全ファイル。深い段も)', [
    fs.readFileSync(path.join(out, 'Plugins/Fruity/Effects/DUP.dll'), 'utf8'),
    fs.readFileSync(path.join(out, 'Plugins/Fruity/Effects/Fruity Balance_x64.dll'), 'utf8'),
    fs.readFileSync(path.join(out, 'Data/System/Sub/deep.txt'), 'utf8'),
  ], ['new', 'balance dll', 'deep']);
  const manifest = fs.readFileSync(path.join(out, 'MANIFEST.tsv'), 'utf8').split('\n').filter((l) => l !== '');
  const rowsWant = [
    ['Artwork\\Themes\\Ash.flstheme', 'theme'],
    ['Data\\System\\Empty.flp', 'project'],
    ['Data\\System\\Sub\\deep.txt', 'deep'],
    ['Plugins\\Fruity\\Effects\\DUP.dll', 'new'],
    ['Plugins\\Fruity\\Effects\\Fruity Balance_x64.dll', 'balance dll'],
    ['Plugins\\Fruity\\Effects\\Readme.txt', 'a file of the prefix, not a DLL'],
  ].map(([g, s]) => [g, String(Buffer.byteLength(s)), sha256(Buffer.from(s))].join('\t'));
  ok('MANIFEST: 見出し(guest・bytes・sha256)の次に、書いた物を guest の小文字の順で', manifest, ['guest\tbytes\tsha256', ...rowsWant]);
  ok('CLI: 接頭辞に無い物(隣の名前・Shared・インストール直下・$PLUGINSDIR)は書かない', listed.filter((f) => /Other|x[.]png|Program[.]exe|tmp[.]dll/.test(f)), []);
  ok('CLI: 出力にも MANIFEST にも標準出力・標準エラーにも、絶対パス(ドライブ名・会社名の段)が無い', [
    /[A-Za-z]:/.test(fs.readFileSync(path.join(out, 'MANIFEST.tsv'), 'utf8')) || listed.some((f) => /:/.test(f)),
    /Program Files|Image-Line/.test(run.stdout + run.stderr + fs.readFileSync(path.join(out, 'MANIFEST.tsv'), 'utf8')),
  ], [false, false]);
  const cli = spawnSync(process.execPath, [TOOL], { encoding: 'utf8' });
  ok('CLI: 引数が無ければ使い方を出し、終了コード 64', cli.status, 64);
  const noSlash = spawnSync(process.execPath, [TOOL, inst, path.join(T, 'o2'), 'Artwork'], { encoding: 'utf8' });
  ok('CLI: 接頭辞が \\ で終わらなければ、終了コード 64 で、何も書かない', [noSlash.status, fs.existsSync(path.join(T, 'o2'))], [64, false]);
  const none = spawnSync(process.execPath, [TOOL, inst, path.join(T, 'o3'), 'Data\\Projects\\', 'Help\\'], { encoding: 'utf8' });
  ok('CLI: 一致が 1 件も無ければ 1 で終わり、何も書かない', [none.status, fs.existsSync(path.join(T, 'o3'))], [1, false]);
  ok('CLI: 一致が無い接頭辞は、接頭辞ごとに警告を出す(2 件)', none.stderr.split('\n').filter((l) => l.includes('警告')).length, 2);
  const bad = spawnSync(process.execPath, [TOOL, path.join(T, 'missing.exe'), path.join(T, 'o4'), 'Artwork\\'], { encoding: 'utf8' });
  ok('CLI: インストーラが無ければ 0 以外で終わる', bad.status !== 0, true);
  // --file GUEST: インストール先の相対の完全なパス(根の直下の物を選ぶ)。接頭辞の規則(\ で終わる)は変えない
  const exactOut = path.join(T, 'o5');
  const exact = spawnSync(process.execPath, [TOOL, inst, exactOut, '--file', 'Program.exe', '--file', 'artwork\\THEMES\\ash.flstheme'], { encoding: 'utf8' });
  ok('CLI --file: 完全なパス(大文字・小文字を区別しない)を指定すれば終了コード 0', exact.status, 0);
  const exactListed = fs.readdirSync(exactOut, { recursive: true }).filter((f) => fs.statSync(path.join(exactOut, f)).isFile()).sort();
  ok('CLI --file: 書かれた物は、指定の完全なパスの物と MANIFEST だけ', exactListed, ['Artwork/Themes/Ash.flstheme', 'MANIFEST.tsv', 'Program.exe']);
  ok('CLI --file: 中身はインストーラの物と同じ', [fs.readFileSync(path.join(exactOut, 'Program.exe'), 'utf8'), fs.readFileSync(path.join(exactOut, 'Artwork/Themes/Ash.flstheme'), 'utf8')], ['MZ outside', 'theme']);
  const exactNone = spawnSync(process.execPath, [TOOL, inst, path.join(T, 'o6'), '--file', 'Nope.exe'], { encoding: 'utf8' });
  ok('CLI --file: 一致が無ければ 1 で終わり、何も書かず、警告を 1 件出す', [exactNone.status, fs.existsSync(path.join(T, 'o6')), exactNone.stderr.split('\n').filter((l) => l.includes('警告') && l.includes('Nope.exe')).length], [1, false, 1]);
  const exactSlash = spawnSync(process.execPath, [TOOL, inst, path.join(T, 'o7'), '--file', 'Artwork\\'], { encoding: 'utf8' });
  ok('CLI --file: 末尾が \\ の物は完全なパスではないので、終了コード 64 で、何も書かない', [exactSlash.status, fs.existsSync(path.join(T, 'o7'))], [64, false]);
  const exactNoValue = spawnSync(process.execPath, [TOOL, inst, path.join(T, 'o8'), '--file'], { encoding: 'utf8' });
  ok('CLI --file: 値が無ければ、終了コード 64 で、何も書かない', [exactNoValue.status, fs.existsSync(path.join(T, 'o8'))], [64, false]);
} finally {
  fs.rmSync(T, { recursive: true, force: true });
}

process.exitCode = failed ? 1 : 0;
