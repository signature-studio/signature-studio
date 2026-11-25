#!/usr/bin/env node
// spec/boot-bundle.test.mjs : tools/boot-check/boot-bundle.mjs の 2 回の起動(ウォームアップ)の合成を、合成の結果と偽の起動で確かめる。2 回目の値を出す(elapsedMs と他の項目)、1 回目を coldMs とする、1 回目の停止を error として出す(2 回目の error より先)、1 回目が 60,000 ms を超えたら「cold boot too slow」、使い方の誤りは終了コード 2。
// 実際の Chrome は使わない(bootTwice に偽の起動を渡す)。終了コード 0 = 全部通った。
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/boot-check/boot-bundle.mjs');
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};
const check = (name, fn, want) => {
  try { ok(name, fn(), want); } catch (e) {
    failed++;
    console.log(`FAIL ${name}: ${e.message}`);
  }
};
// one start's JSON, as bootBundle's out has it (the fields the check prints)
const start = (state, elapsedMs, extra = {}) => ({
  startedAt: '2026-11-25T00:00:00.000Z', state, engineKind: 'k', timelineIdle: elapsedMs, engineLog30: [], standIn: false,
  documentTitle: 't', elapsedMs, loadavg: [0, 0, 0], chromeVersion: 'c', error: null, ...extra,
});
const COLD = start('engine-idle', 17714);
const WARM = start('engine-idle', 11494, { timelineIdle: 11400, engineKind: 'warm-kind' });

const { composeBoot, bootTwice } = await import(pathToFileURL(TOOL).href).catch((e) => ({ composeBoot: undefined, bootTwice: undefined, importError: e }));

check('合成: elapsedMs は 2 回目の値(11494)', () => composeBoot(COLD, WARM).elapsedMs, 11494);
check('合成: coldMs は 1 回目の elapsedMs(17714)', () => composeBoot(COLD, WARM).coldMs, 17714);
check('合成: state・error・standIn は 2 回目の値で、1 回目が正常なら error は null', () => {
  const o = composeBoot(COLD, WARM);
  return [o.state, o.error, o.standIn];
}, ['engine-idle', null, false]);
check('合成: 他の項目(timelineIdle・engineKind)も 2 回目の値', () => {
  const o = composeBoot(COLD, WARM);
  return [o.timelineIdle, o.engineKind];
}, [11400, 'warm-kind']);
check('合成: 1 回目の error は、そのまま error として出る(2 回目が正常でも)', () => {
  const o = composeBoot(start('error', 3000, { error: 'x' }), WARM);
  return [o.error, o.coldMs];
}, ['x', 3000]);
check('合成: 1 回目の error が null でも state が engine-idle でなければ、失敗(error が出る)', () => composeBoot(start('error', 3000), WARM).error.includes('cold start: state error'), true);
check('合成: 1 回目が exited でも失敗', () => composeBoot(start('exited', 3000), WARM).error.includes('cold start: state exited'), true);
check('合成: 1 回目と 2 回目が両方失敗なら、1 回目の error が出る', () => composeBoot(start('error', 3000, { error: 'x' }), start('error', 2000, { error: 'y' })).error, 'x');
check('合成: 1 回目は正常で 2 回目が失敗なら、2 回目の error が出る', () => composeBoot(COLD, start('error', 2000, { error: 'y' })).error, 'y');
check('合成: 2 回目の state が idle でなくても、2 回目の値を、そのまま出す', () => composeBoot(COLD, start('error', 2000)).state, 'error');
check('合成: 1 回目が 60,000 ms ちょうどは遅すぎない(error null)', () => composeBoot(start('engine-idle', 60000), WARM).error, null);
check('合成: 1 回目が 60,001 ms は遅すぎる(cold boot too slow)', () => composeBoot(start('engine-idle', 60001), WARM).error.includes('cold boot too slow'), true);
check('合成: 1 回目の elapsedMs が null(起動できなかった)なら、coldMs は null で error は 1 回目の error', () => {
  const o = composeBoot(start('error', null, { error: 'launch failed' }), WARM);
  return [o.coldMs, o.error];
}, [null, 'launch failed']);

const queue = (outs, calls) => (dir) => {
  calls.push(dir);
  return Promise.resolve({ out: outs[calls.length - 1] });
};
for (const [name, outs, pick, want] of [
  ['bootTwice: 同じ bundle を 2 回起動し、2 回目の elapsedMs と、1 回目の coldMs を出す', [COLD, WARM], (o) => [o.elapsedMs, o.coldMs], [11494, 17714]],
  ['bootTwice: 1 回目が失敗しても 2 回目も起動し、error は 1 回目のもの', [start('error', 3000, { error: 'x' }), WARM], (o) => [o.coldMs, o.error], [3000, 'x']],
]) {
  const calls = [];
  try {
    const o = await bootTwice('/bundle/raw/fl-Oz', queue(outs, calls));
    ok(name, [calls.length, ...pick(o)], [2, ...want]);
  } catch (e) {
    failed++;
    console.log(`FAIL ${name}: ${e.message}`);
  }
}

check('CLI: 引数が無ければ、使い方を出して終了コード 2(Chrome は起動しない)', () => spawnSync(process.execPath, [TOOL], { encoding: 'utf8' }).status, 2);

console.log(failed === 0 ? 'all passed' : `${failed} failed`);
process.exitCode = failed === 0 ? 0 : 1;
