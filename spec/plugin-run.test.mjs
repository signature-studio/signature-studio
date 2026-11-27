#!/usr/bin/env node
// spec/plugin-run.test.mjs : tools/plugin-run/run.mjs (Issue #1371)。名前の解決(lift/plugins の効果のフォルダ、大文字・空白・ハイフンを区別しない)、
// 存在しない名前と効果でない名前で 1 で止まり、標準出力に何も出さないこと、読みの一致(分離 wasm の名前)、.flp の選び方を確かめる。Chrome は使わない。
// 一時ディレクトリはリポジトリの中の .tmp-*(git の無視の対象)に作り、終わったら消す。終了コード 0 = 全部通った。
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/plugin-run/run.mjs');
let failed = 0;
const ok = (name, cond, detail = '') => {
  if (cond) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}${detail ? `: ${detail}` : ''}`);
  }
};

let m;
try {
  m = await import(pathToFileURL(TOOL).href);
} catch (e) {
  console.log(`FAIL 道具が読めない: ${e.message.split('\n')[0]}`);
  process.exit(1);
}
const { keyOf, effectRows, resolveNames, projectFor, wasmReadRe, isPlayFailure, playFailuresOf } = m;

// 1) 存在しない名前は 1 で止まる(標準出力は空。バンドルの場所は読まれないので、在るディレクトリを渡す)
const run = (args) => spawnSync(process.execPath, [TOOL, ...args], { encoding: 'utf8', timeout: 60000 });
const miss = run([ROOT, 'No Such Plugin']);
ok('存在しない名前で終了コード 1', miss.status === 1, `status ${miss.status}`);
ok('存在しない名前で標準出力が空', miss.stdout === '', JSON.stringify(miss.stdout));
ok('存在しない名前の理由が標準エラーに出る', /no effect plug-in named "No Such Plugin"/.test(miss.stderr), miss.stderr.slice(0, 200));

// 2) 効果でない名前(インストゥルメント)も 1 で止まる
const gen = run([ROOT, 'Sytrus']);
ok('インストゥルメント(Sytrus)で終了コード 1', gen.status === 1, `status ${gen.status}`);
ok('インストゥルメントで標準出力が空', gen.stdout === '', JSON.stringify(gen.stdout));

// 3) 名前が無いときは使い方の終了コード 64
const usage = run([ROOT]);
ok('名前が無いときは終了コード 64', usage.status === 64, `status ${usage.status}`);

// 4) 名前の比べ方: 大文字・空白・ハイフンを区別しない
ok('keyOf: "Fruity Balance" と "fruity-balance" は同じ', keyOf('Fruity Balance') === keyOf('fruity-balance'));
ok('keyOf: 大文字と小文字を区別しない', keyOf('FRUITY-FILTER') === keyOf('fruity filter'));

// 5) 効果の行: lift/plugins の本体(role 本体、key effects.*)の行。Balance と Filter が在り、Sytrus(インストゥルメント)は無い
const rows = effectRows();
const byName = Object.fromEntries(rows.map((r) => [r.name, r]));
ok('効果の行に Fruity Balance と Fruity Filter が在る', 'Fruity Balance' in byName && 'Fruity Filter' in byName, JSON.stringify(rows));
ok('効果の行の stem は DLL の key から .dll を除いたもの', byName['Fruity Balance']?.stem === 'effects.fruity-balance.fruity_balance_x64', JSON.stringify(byName['Fruity Balance']));
ok('効果の行に、インストゥルメントは無い', rows.every((r) => r.key.startsWith('effects.')));

// 6) 名前の解決
const got = resolveNames(['Fruity Balance', 'fruity-filter']);
ok('resolveNames: 2 つの名前が 2 行になる', got.length === 2 && got[0].folder === 'fruity-balance' && got[1].folder === 'fruity-filter', JSON.stringify(got));
let threw = false;
try { resolveNames(['No Such Plugin']); } catch { threw = true; }
ok('resolveNames: 存在しない名前は投げる', threw);

// 7) 分離 wasm の読み(image-read の行の名前)は、本体の stem の後に任意の基底と .wasm
const re = wasmReadRe('effects.fruity-balance.fruity_balance_x64');
ok('wasmReadRe: 基底つきの wasm の名前に一致する', re.test('effects.fruity-balance.fruity_balance_x64.c1d0000.wasm'));
ok('wasmReadRe: DLL の名前には一致しない', !re.test('effects.fruity-balance.fruity_balance_x64.c1d0000.dll'));
ok('wasmReadRe: 別の本体には一致しない', !re.test('effects.fruity-filter.fruity_filter_x64.c580000.wasm'));

// 8) .flp の選び方: UTF-16LE の名前を含む物。根の順(host-extra が先)。同じ根では最も小さい物
const tmp = fs.mkdtempSync(path.join(ROOT, '.tmp-spec-plugin-run.'));
try {
  const u16 = (s) => Buffer.from(s, 'utf16le');
  const a = path.join(tmp, 'a'), b = path.join(tmp, 'b'), c = path.join(tmp, 'c');
  fs.mkdirSync(path.join(a, 'sub'), { recursive: true });
  fs.mkdirSync(b, { recursive: true });
  fs.mkdirSync(c, { recursive: true });
  fs.writeFileSync(path.join(a, 'sub', 'big.flp'), Buffer.concat([Buffer.alloc(4000), u16('Fruity Test')]));
  fs.writeFileSync(path.join(a, 'small.flp'), Buffer.concat([Buffer.alloc(100), u16('Fruity Test')]));
  fs.writeFileSync(path.join(a, 'ascii-only.flp'), Buffer.from('Fruity Test in ASCII only'));
  fs.writeFileSync(path.join(b, 'other.flp'), Buffer.concat([Buffer.alloc(10), u16('Fruity Test')]));
  fs.writeFileSync(path.join(c, 'none.flp'), Buffer.alloc(10));
  const pick = projectFor('Fruity Test', [a, b]);
  ok('projectFor: 同じ根では、UTF-16LE の名前を含む最も小さい .flp', pick === path.join(a, 'small.flp'), String(pick));
  ok('projectFor: 先の根に候補が在れば、後の根は見ない', projectFor('Fruity Test', [b, a]) === path.join(b, 'other.flp'), String(projectFor('Fruity Test', [b, a])));
  ok('projectFor: ASCII だけの名前は選ばない', projectFor('Fruity Test', [c]) === null);
  ok('projectFor: 名前の無い根は null', projectFor('Nothing Here', [a, b, c]) === null);
  ok('projectFor: 存在しない根は飛ばす', projectFor('Fruity Test', [path.join(tmp, 'missing'), a]) === path.join(a, 'small.flp'));
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

// 9) 再生の判定(Issue #1371): 失敗の行は、スレッドの失敗・サスペンドの誤り・エンジンの停止。それ以外の行は失敗ではない
ok('isPlayFailure: fle thread の失敗の行', isPlayFailure('[err] [js] fle thread 23 failed: SuspendError: trying to suspend JS frames'));
ok('isPlayFailure: 停止の行', isPlayFailure('fruity: runtime message: fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no structured exception handling is modelled, stopped here'));
ok('isPlayFailure: メモリの行は失敗ではない', !isPlayFailure('[err] memory-usage: K32GetProcessMemoryInfo WorkingSetSize 198091964'));
ok('playFailuresOf: 再生が増え、失敗の行が無く、engine-idle なら空', playFailuresOf({ lines: ['[err] missing-path x'], state: 'engine-idle', quanta: 400 }).length === 0);
const fl = playFailuresOf({ lines: ['[err] [js] fle thread 23 failed: SuspendError: x', 'ok line'], state: 'engine-idle', quanta: 400 });
ok('playFailuresOf: 失敗の行だけを入れる', fl.length === 1 && fl[0].includes('SuspendError'), JSON.stringify(fl));
ok('playFailuresOf: quantaWhilePlaying が増えないと失敗', playFailuresOf({ lines: [], state: 'engine-idle', quanta: 0 }).length === 1);
ok('playFailuresOf: quantaWhilePlaying が測れないと失敗', playFailuresOf({ lines: [], state: 'engine-idle', quanta: null }).length === 1);
const ex = playFailuresOf({ lines: [], state: 'exited', stateNote: ' (x [y])', quanta: 400 });
ok('playFailuresOf: 再生の後に exited なら、状態の行が入る', ex.length === 1 && ex[0] === 'state exited after the play (x [y])', JSON.stringify(ex));

console.log(failed === 0 ? 'ok   spec/plugin-run.test.mjs: 全部通った' : `FAIL spec/plugin-run.test.mjs: ${failed} 件`);
process.exit(failed === 0 ? 0 : 1);
