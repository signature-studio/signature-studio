#!/usr/bin/env node
// spec/external-inputs.test.mjs : tools/external-inputs/check.mjs(外部プラグインのインストーラの照合)を確かめる。偽の lock と小さな偽のファイルで試す。
// 一時ファイルはリポジトリの .tmp-spec-ext-* に作り、終わったら消す。終了コード 0 = 全部通った。
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/external-inputs/check.mjs');
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const sha256 = (b) => crypto.createHash('sha256').update(b).digest('hex');

const SERUM = Buffer.from('serum installer bytes');
const OTT = Buffer.from('ott installer bytes');
const VITAL = Buffer.from('vital installer bytes');
const lockText = (rows) => `# test lock\n${rows.join('\n')}\n`;
const row = (key, data, used) => `${key}\t${data.length}\t${sha256(data)}\td41d8\t${used}`;
const LOCK = [
  row('installers/Install_Xfer_Serum2_2.1.5.exe', SERUM, 'Serum 2 2.1.5 (VST3)'),
  row('installers/Install_Xfer_OTT_137.exe', OTT, 'OTT 1.37 (VST3)'),
  row('installers/VitalInstaller.exe', VITAL, 'Vital (VST3)'),
];
const FL = `installers/flstudio_win64_26.1.7.5653.exe\t999\t${'0'.repeat(64)}\td41d8\tFL Studio`;

const T = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-ext-'));
const run = (args) => spawnSync(process.execPath, [TOOL, ...args], { encoding: 'utf8' });
const putDir = (name, files) => {
  const dir = path.join(T, name);
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  for (const [f, data] of Object.entries(files)) fs.writeFileSync(path.join(dir, f), data);
  return dir;
};
const lock = path.join(T, 'artifacts.lock');
fs.writeFileSync(lock, lockText([FL, ...LOCK]));
const json = (r) => {
  try {
    return JSON.parse(r.stdout.trim());
  } catch {
    return null;
  }
};

try {
  const good = putDir('good', {
    'Install_Xfer_Serum2_2.1.5.exe': SERUM,
    'Install_Xfer_OTT_137.exe': OTT,
    'VitalInstaller.exe': VITAL,
  });
  const g = run([good, '--lock', lock]);
  ok('全件が一致するとき、終了コード 0', g.status, 0);
  ok('全件が一致するとき、JSON の 1 行で ok:true。外部の 3 件だけ(FL は数えない)', [json(g)?.ok, json(g)?.files.length, g.stdout.trim().split('\n').length], [true, 3, 1]);
  ok('各件の used は lock の説明。reason は ok', json(g)?.files.map((f) => [f.used, f.ok, f.reason]), [
    ['Serum 2 2.1.5 (VST3)', true, 'ok'],
    ['OTT 1.37 (VST3)', true, 'ok'],
    ['Vital (VST3)', true, 'ok'],
  ]);
  ok('出力にファイル名もパスも出ない', [g.stdout.includes('Install_Xfer'), g.stdout.includes('VitalInstaller'), g.stdout.includes(T)], [false, false, false]);

  const missing = putDir('missing', { 'Install_Xfer_Serum2_2.1.5.exe': SERUM, 'Install_Xfer_OTT_137.exe': OTT });
  const m1 = run([missing, '--lock', lock]);
  ok('ファイルが無ければ終了コード 1、reason は missing', [m1.status, json(m1)?.ok, json(m1)?.files.at(-1).reason], [1, false, 'missing']);

  const sized = putDir('sized', { 'Install_Xfer_Serum2_2.1.5.exe': SERUM, 'Install_Xfer_OTT_137.exe': OTT, 'VitalInstaller.exe': Buffer.from('vital, longer') });
  const s1 = run([sized, '--lock', lock]);
  ok('大きさが違えば reason は size(SHA-256 は見ない)', [s1.status, json(s1)?.files.at(-1).reason], [1, 'size']);

  const same = putDir('same', { 'Install_Xfer_Serum2_2.1.5.exe': SERUM, 'Install_Xfer_OTT_137.exe': OTT, 'VitalInstaller.exe': Buffer.from('X'.repeat(VITAL.length)) });
  const h1 = run([same, '--lock', lock]);
  ok('大きさが同じで中身が違えば reason は sha256', [h1.status, json(h1)?.files.map((f) => f.reason)], [1, ['ok', 'ok', 'sha256']]);

  const none = run([]);
  ok('引数が無ければ終了コード 64', none.status, 64);

  const real = run([path.join(T, 'empty')]);
  ok('lock を省くと ci/artifacts.lock を読む(空の置き場所なので 3 件とも missing)', [real.status, json(real)?.files.map((f) => [f.used, f.reason])], [1, [
    ['Serum 2 2.1.5 (VST3)', 'missing'],
    ['OTT 1.37 (VST3)', 'missing'],
    ['Vital (VST3)', 'missing'],
  ]]);
} finally {
  fs.rmSync(T, { recursive: true, force: true });
}

process.exitCode = failed ? 1 : 0;
