#!/usr/bin/env node
// spec/boot-bundle.test.mjs : tools/boot-check/boot-bundle.mjs の起動の合成を、合成の結果と偽の起動で確かめる。1 回目は冷たい起動(coldMs)、続く 3 回は温かい起動。elapsedMs は温かい 3 回の中央値(その run の他の項目も、その run の値)、samples は温かい 3 回の elapsedMs を測った順に出す。error は 1 回目の停止、次に温かい 3 回のどれかの停止(1 回でも error・idle でない・STAND-IN なら、その error)。1 回目が 60,000 ms を超えたら「cold boot too slow」。使い方の誤りは終了コード 2。
// 実際の Chrome は使わない(bootSeries に偽の起動を渡す)。終了コード 0 = 全部通った。
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
const WARMS = [WARM, WARM, WARM];

const { composeBoot, bootSeries } = await import(pathToFileURL(TOOL).href).catch((e) => ({ composeBoot: undefined, bootSeries: undefined, importError: e }));

check('合成: elapsedMs は温かい 3 回の中央値(11494)', () => composeBoot(COLD, WARMS).elapsedMs, 11494);
check('合成: coldMs は 1 回目の elapsedMs(17714)', () => composeBoot(COLD, WARMS).coldMs, 17714);
check('合成: state・error・standIn は、温かい 3 回が正常なら、中央値の run の値で、error は null', () => {
  const o = composeBoot(COLD, WARMS);
  return [o.state, o.error, o.standIn];
}, ['engine-idle', null, false]);
check('合成: 他の項目(timelineIdle・engineKind)も、中央値の run の値', () => {
  const o = composeBoot(COLD, WARMS);
  return [o.timelineIdle, o.engineKind];
}, [11400, 'warm-kind']);
check('合成: 1 回目の error は、そのまま error として出る(温かい 3 回が正常でも)', () => {
  const o = composeBoot(start('error', 3000, { error: 'x' }), WARMS);
  return [o.error, o.coldMs];
}, ['x', 3000]);
check('合成: 1 回目の error が null でも state が engine-idle でなければ、失敗(error が出る)', () => composeBoot(start('error', 3000), WARMS).error.includes('cold start: state error'), true);
check('合成: 1 回目が exited でも失敗', () => composeBoot(start('exited', 3000), WARMS).error.includes('cold start: state exited'), true);
check('合成: 1 回目と温かい 1 回目が両方失敗なら、1 回目の error が出る', () => composeBoot(start('error', 3000, { error: 'x' }), [start('error', 2000, { error: 'y' }), WARM, WARM]).error, 'x');
check('合成: 1 回目は正常で、温かい 1 回(2 回目)が失敗すれば、その error が出る', () => composeBoot(COLD, [WARM, start('error', 2000, { error: 'y' }), WARM]).error, 'y');
check('合成: 温かい 1 回(3 回目)が idle でなければ、その state の error(中央値は正常でも)', () => composeBoot(COLD, [WARM, WARM, start('error', 2000)]).error, 'warm start: state error');
check('合成: 温かい 1 回が STAND-IN なら、error に STAND-IN が出る', () => composeBoot(COLD, [WARM, start('engine-idle', 11000, { standIn: true }), WARM]).error, 'warm start: STAND-IN');
check('合成: 1 回目が 60,000 ms ちょうどは遅すぎない(error null)', () => composeBoot(start('engine-idle', 60000), WARMS).error, null);
check('合成: 1 回目が 60,001 ms は遅すぎる(cold boot too slow)', () => composeBoot(start('engine-idle', 60001), WARMS).error.includes('cold boot too slow'), true);
check('合成: 1 回目の elapsedMs が null(起動できなかった)なら、coldMs は null で error は 1 回目の error', () => {
  const o = composeBoot(start('error', null, { error: 'launch failed' }), WARMS);
  return [o.coldMs, o.error];
}, [null, 'launch failed']);
check('合成: samples は温かい 3 回の elapsedMs を、測った順に出す', () => composeBoot(COLD, [start('engine-idle', 12001), start('engine-idle', 10900), start('engine-idle', 11494)]).samples, [12001, 10900, 11494]);
check('合成: elapsedMs は 3 回の中央値(12001・10900・11494 の中央は 11494)', () => composeBoot(COLD, [start('engine-idle', 12001), start('engine-idle', 10900), start('engine-idle', 11494)]).elapsedMs, 11494);
check('合成: 1 回だけ 12,500 を超えた揺れ(12573)は、中央値に出ない(11000・11200 の中央は 11200)', () => composeBoot(COLD, [start('engine-idle', 12573), start('engine-idle', 11000), start('engine-idle', 11200)]).elapsedMs, 11200);
check('合成: 中央値の run の他の項目(timelineIdle)も、その run の値', () => composeBoot(COLD, [start('engine-idle', 12573, { timelineIdle: 1 }), start('engine-idle', 11000, { timelineIdle: 2 }), start('engine-idle', 11200, { timelineIdle: 3 })]).timelineIdle, 3);

const queue = (outs, calls) => (dir) => {
  calls.push(dir);
  return Promise.resolve({ out: outs[calls.length - 1] });
};
for (const [name, outs, pick, want] of [
  ['bootSeries: 同じ bundle を 4 回起動し(1 回目は冷たい、続く 3 回は温かい)、中央値の elapsedMs・1 回目の coldMs・samples の件数を出す', [COLD, WARM, WARM, WARM], (o) => [o.elapsedMs, o.coldMs, o.samples.length], [11494, 17714, 3]],
  ['bootSeries: 1 回目が失敗しても温かい 3 回を続けて起動し、error は 1 回目のもの', [start('error', 3000, { error: 'x' }), WARM, WARM, WARM], (o) => [o.coldMs, o.error], [3000, 'x']],
  ['bootSeries: 温かい 1 回が失敗しても 4 回すべて起動し、error はその温かい 1 回のもの', [COLD, WARM, start('error', 2000, { error: 'y' }), WARM], (o) => [o.error, o.samples.length], ['y', 3]],
]) {
  const calls = [];
  try {
    const o = await bootSeries('/bundle/raw/fl-Oz', queue(outs, calls));
    ok(name, [calls.length, ...pick(o)], [4, ...want]);
  } catch (e) {
    failed++;
    console.log(`FAIL ${name}: ${e.message}`);
  }
}

check('CLI: 引数が無ければ、使い方を出して終了コード 2(Chrome は起動しない)', () => spawnSync(process.execPath, [TOOL], { encoding: 'utf8' }).status, 2);

console.log(failed === 0 ? 'all passed' : `${failed} failed`);
process.exitCode = failed === 0 ? 0 : 1;
