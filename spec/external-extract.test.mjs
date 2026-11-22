#!/usr/bin/env node
// spec/external-extract.test.mjs : tools/external-inputs の取り出しの道具(crc32・lzma2・pluginName・inno・pe)を確かめる。
// 実物(lift/out-app/external の Vital のインストーラ、lift/out-app/external-vst3 の取り出した DLL)が在るときは、それとも照らす。無いときは skip する。
// 終了コード 0 = 全部通った。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { crc32, innoFiles } from '../tools/external-inputs/inno.mjs';
import { lzma2Decode } from '../tools/external-inputs/lzma2.mjs';
import { pluginName } from '../tools/external-inputs/extract.mjs';
import { isPe, peImports } from '../tools/external-inputs/pe.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const sha256 = (b) => crypto.createHash('sha256').update(b).digest('hex');

ok('crc32 は標準の CRC-32(123456789 で 0xCBF43926)', crc32(Buffer.from('123456789')), 0xcbf43926);

const stored = Buffer.concat([
  Buffer.from([0x18]),
  Buffer.from([0x01, 0x00, 0x02]), Buffer.from('abc'),
  Buffer.from([0x02, 0x00, 0x01]), Buffer.from('de'),
  Buffer.from([0x00]),
]);
ok('lzma2: 非圧縮のチャンク(1・2)を並べて展開する', lzma2Decode(stored).toString('latin1'), 'abcde');
ok('lzma2: 制御バイトが不正なら例外', (() => {
  try {
    lzma2Decode(Buffer.from([0x18, 0x03, 0x00]));
    return 'no error';
  } catch (e) {
    return e.message;
  }
})(), 'LZMA2: bad control byte 3');

ok('pluginName は lock の説明から版の数字を除く', ['Serum 2 2.1.5 (VST3)', 'OTT 1.37 (VST3)', 'Vital (VST3)'].map(pluginName), ['Serum 2', 'OTT', 'Vital']);

/** a PE32+ image of one section whose import table names `dll` (or none) */
function synthPe(dll) {
  const b = Buffer.alloc(0x400);
  b.write('MZ', 0, 'latin1');
  b.writeUInt32LE(0x80, 0x3c);
  b.write('PE\0\0', 0x80, 'latin1');
  b.writeUInt16LE(1, 0x86);
  b.writeUInt16LE(0xf0, 0x94);
  b.writeUInt16LE(0x20b, 0x98);
  b.writeUInt32LE(0x1000, 0x110);
  b.writeUInt32LE(40, 0x114);
  const s = 0x188;
  b.writeUInt32LE(0x200, s + 8);
  b.writeUInt32LE(0x1000, s + 12);
  b.writeUInt32LE(0x200, s + 16);
  b.writeUInt32LE(0x200, s + 20);
  b.writeUInt32LE(0x1100, 0x200 + 12);
  b.write(`${dll}\0`, 0x300, 'latin1');
  return b;
}
ok('isPe は MZ と PE\\0\\0 を見る', [isPe(synthPe('x.dll')), isPe(Buffer.from('MZ'))], [true, false]);
ok('peImports は import 表の DLL 名を小文字で返す', peImports(synthPe('KERNEL32.dll')), ['kernel32.dll']);
const none = synthPe('x.dll');
none.writeUInt32LE(0, 0x110);
ok('peImports は import 表の無い PE では空', peImports(none), []);

const innoFile = path.join(ROOT, 'lift/out-app/external/VitalInstaller.exe');
const extracted = path.join(ROOT, 'lift/out-app/external-vst3');
const tsvFile = path.join(ROOT, 'inputs/external-plugins.tsv');
if (!fs.existsSync(innoFile)) {
  console.log('skip Vital のインストーラが無いので、innoFiles は試さない');
} else {
  const files = innoFiles(fs.readFileSync(innoFile));
  ok('innoFiles は Vital の 6 件を、SHA-256 の照合を通して返す', files.map((f) => f.dest), [
    '{app}\\Vital.exe', '{code:GetVST3Dir}\\Vital.vst3', '{code:GetVST64Dir}\\Vital.dll', '{code:GetCLAPDir}\\Vital.clap',
    '{app}\\stub\\vc_redist.x64.exe', '{app}\\installer_icon.ico',
  ]);
  const v3 = files.find((f) => f.dest === '{code:GetVST3Dir}\\Vital.vst3');
  ok('Vital.vst3 は PE で、Authenticode の署名と同じ SHA-256 を持つ', [isPe(v3.data), sha256(v3.data)], [true, 'a622a2c99b4066cd7945a4ab9bbdd698e7632a30702f6f0a7ccbf26a56b576e1']);
}

if (fs.existsSync(tsvFile)) {
  const rows = fs.readFileSync(tsvFile, 'utf8').split('\n').filter((ln) => ln !== '').map((ln) => ln.split('\t'));
  ok('inputs/external-plugins.tsv は見出しと 3 件の Plugin を持つ', [rows[0].join('|'), new Set(rows.slice(1).map((r) => r[0])).size], ['plugin|path|bytes|sha256|imports', 3]);
  ok('TSV の sha256 は 64 桁の 16 進', rows.slice(1).every((r) => /^[0-9a-f]{64}$/.test(r[3])), true);
  if (fs.existsSync(extracted)) {
    const bins = { 'Serum 2': 'serum-2', OTT: 'ott', Vital: 'vital' };
    const got = rows.slice(1).map((r) => {
      const file = path.join(extracted, bins[r[0]], path.posix.basename(r[1]));
      const b = fs.existsSync(file) ? fs.readFileSync(file) : null;
      return b && [b.length, sha256(b), peImports(b).join(';')].join('\t') === [r[2], r[3], r[4]].join('\t') ? 'match' : 'differs';
    });
    ok('TSV の各行は、取り出した DLL の大きさ・sha256・imports と一致する', got, ['match', 'match', 'match']);
  } else {
    console.log('skip lift/out-app/external-vst3 が無いので、TSV と取り出した DLL を照らさない');
  }
} else {
  console.log('skip inputs/external-plugins.tsv が無い');
}

process.exitCode = failed ? 1 : 0;
