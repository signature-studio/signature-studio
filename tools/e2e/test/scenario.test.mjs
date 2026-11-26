// The registry of scenarios (lib/scenario.mjs) and the run's plan (lib/plan.mjs): what a scenario must declare, the shards, the budget, the displays and the acceptance.
import test from 'node:test';
import assert from 'node:assert/strict';
import { AREAS, areaOf, checkMark, checkMarks, defineScenario, legacyScenario, marksOf, shardOf } from '../lib/scenario.mjs';
import { budgetClock, displayRows, parseArgs, parseBudget, parseRepeat, parseShard, planScenarios, rowPasses } from '../lib/plan.mjs';

const ok = { kind: 'scenario', run: () => ({ ok: true }) };
const sc = (id, extra = {}) => defineScenario({ id, run: ok.run, ...extra });

test('the 13 areas are the ones the issue names', () => {
  assert.deepEqual(AREAS, ['page', 'input', 'file', 'menus', 'windows', 'audio', 'settings', 'plugins', 'topbar', 'resilience', 'visual', 'perf', 'delivery']);
});

test('defineScenario takes the id, the area from it, and defaults displays and expectFail', () => {
  const s = sc('page/canvas-fills-viewport', { covers: ['page/module/resize'] });
  assert.equal(s.area, 'page');
  assert.deepEqual(s.covers, ['page/module/resize']);
  assert.deepEqual(s.displays, ['4k-dpr1', '4k-random']);
  assert.equal(s.expectFail, null);
  assert.equal(s.kind, 'scenario');
});

test('defineScenario rejects a bad id, an unknown area, a cover of no area, a repeated cover and a bad mark', () => {
  assert.throws(() => sc('Page/x'), /scenario id/);
  assert.throws(() => sc('nope/x'), /area nope/);
  assert.throws(() => sc('page/x', { covers: ['page'] }), /cover/);
  assert.throws(() => sc('page/x', { covers: ['page/module/a', 'page/module/a'] }), /repeats/);
  assert.throws(() => sc('page/x', { expectFail: '1421' }), /expectFail/);
  assert.throws(() => sc('page/x', { displays: [] }), /displays/);
  assert.throws(() => sc('page/x', { run: 1 }), /run/);
});

test('an expectFail mark is null, #<number> or new', () => {
  assert.equal(checkMark(null), null);
  assert.equal(checkMark('#1421'), '#1421');
  assert.equal(checkMark('new'), 'new');
  assert.throws(() => checkMark('#abc'));
});

test('a list of marks: each is a mark; checkMarks refuses an issue outside the allowed list, and takes new', () => {
  assert.deepEqual(checkMark(['#1395', '#1419']), ['#1395', '#1419']);
  assert.throws(() => checkMark([]), /empty/);
  assert.throws(() => checkMark(['#1', 'x']), /expectFail/);
  assert.deepEqual(marksOf(null), []);
  assert.deepEqual(marksOf('#1'), ['#1']);
  assert.deepEqual(marksOf(['#1', '#2']), ['#1', '#2']);
  assert.throws(() => checkMarks([{ name: 'a', expectFail: '#1' }], ['#1502']), /allowed/);
  checkMarks([{ name: 'b', expectFail: ['#1502', 'new'] }, { name: 'c', expectFail: null }], ['#1502']);
  checkMarks([{ name: 'd', expectFail: 'new' }], null);
});

test('areaOf reads the area of a scenario ID or a feature ID', () => {
  assert.equal(areaOf('menus/item/file-1'), 'menus');
  assert.equal(areaOf('smoke/x'), null);
  assert.equal(areaOf('nothing'), null);
});

test('a legacy scenario keeps its name and runs in a page of its own', () => {
  const l = legacyScenario({ name: 'file-open-demo', expectFail: '#1421', run: () => ({ ok: true }) });
  assert.equal(l.kind, 'legacy');
  assert.equal(l.id, 'file-open-demo');
  assert.equal(l.expectFail, '#1421');
  assert.deepEqual(l.displays, ['4k-dpr1', '4k-dpr2', '4k-random']);
  assert.equal(l.area, 'smoke');
  assert.equal(legacyScenario({ name: 'browser-face-tabs', expectFail: null, run: () => ({ ok: true }) }, 'browser-face').area, 'browser-face');
});

test('a legacy scenario keeps its beforeLoad (the permission and the counter are set before the page loads)', () => {
  const beforeLoad = async () => {};
  const l = legacyScenario({ name: 'clipboard-no-toast', expectFail: null, beforeLoad, run: () => ({ ok: true }) });
  assert.equal(l.beforeLoad, beforeLoad);
});

test('the smoke set keeps the beforeLoad of its two clipboard scenarios as legacy scenarios', async () => {
  const smoke = (await import('../scenarios/smoke/index.mjs')).default;
  assert.deepEqual(smoke.filter((s) => s.beforeLoad).map((s) => s.name).sort(), ['clipboard-load-permission', 'clipboard-no-toast']);
  for (const s of smoke) assert.equal(legacyScenario(s).beforeLoad, s.beforeLoad, s.name);
});

test('shards are deterministic: the sorted index modulo n', () => {
  const list = ['page/c', 'page/a', 'page/b', 'page/d', 'page/e'].map((id) => ({ id }));
  assert.deepEqual(shardOf(list, 1, 2).map((s) => s.id), ['page/a', 'page/c', 'page/e']);
  assert.deepEqual(shardOf(list, 2, 2).map((s) => s.id), ['page/b', 'page/d']);
});

test('planScenarios sorts, shards, repeats and filters by ID or name', () => {
  const all = [{ id: 'page/b', name: 'page/b' }, { id: 'page/a', name: 'page/a' }, { id: 'old-name', name: 'old-name' }];
  assert.deepEqual(planScenarios(all, { only: ['old-name'] }).map((p) => p.sc.id), ['old-name']);
  assert.deepEqual(planScenarios(all, { only: ['page/b'] }).map((p) => p.sc.id), ['page/b']);
  const rep = planScenarios(all, { shard: { i: 1, n: 1 }, repeat: 2 });
  assert.deepEqual(rep.map((p) => `${p.sc.id}#${p.repeat}`), ['old-name#1', 'page/a#1', 'page/b#1', 'old-name#2', 'page/a#2', 'page/b#2']);
  assert.throws(() => planScenarios(all, { only: ['nope'] }), /--only/);
});

test('arguments: pairs, the list flag, and the errors', () => {
  assert.deepEqual(parseArgs(['--area', 'page', '--list', '--shard', '1/2']), { area: 'page', list: true, shard: '1/2' });
  assert.throws(() => parseArgs(['page']), /unexpected/);
  assert.throws(() => parseArgs(['--area']), /needs a value/);
});

test('budgets: s, m and h', () => {
  assert.equal(parseBudget('40m'), 2400000);
  assert.equal(parseBudget('90s'), 90000);
  assert.equal(parseBudget('2h'), 7200000);
  assert.throws(() => parseBudget('40'), /budget/);
  assert.throws(() => parseBudget('4x'), /budget/);
});

test('the budget clock: expired once the budget has passed', () => {
  let t = 1000;
  const c = budgetClock(500, () => t);
  assert.equal(c.expired(), false);
  t = 1500;
  assert.equal(c.expired(), true);
  assert.equal(budgetClock(null, () => t).expired(), false);
});

test('shard and repeat arguments', () => {
  assert.deepEqual(parseShard('2/3'), { i: 2, n: 3 });
  assert.throws(() => parseShard('3/2'), /shard/);
  assert.throws(() => parseShard('0/2'), /shard/);
  assert.equal(parseRepeat('3'), 3);
  assert.throws(() => parseRepeat('0'), /repeat/);
});

test('displays: the three names and dpr:<number>, the CSS size of 3840 x 2160 at the DPR', () => {
  assert.deepEqual(displayRows('4k-dpr1'), [{ name: '4k-dpr1', dpr: 1, cssWidth: 3840, cssHeight: 2160 }]);
  assert.deepEqual(displayRows('4k-dpr2'), [{ name: '4k-dpr2', dpr: 2, cssWidth: 1920, cssHeight: 1080 }]);
  assert.deepEqual(displayRows('dpr:1.25'), [{ name: 'dpr:1.25', dpr: 1.25, cssWidth: 3072, cssHeight: 1728 }]);
  assert.deepEqual(displayRows('4k-random', () => 0).map((r) => r.dpr), [1.01, 1.01]);
  assert.throws(() => displayRows('dpr:9'), /DPR/);
  assert.throws(() => displayRows('8k'), /no display/);
});

test('rowPasses: a passing row, a skipped scenario, a failing one, and a mark scenario', () => {
  const good = { servedHeadersMatch: true, error: null, scenarios: [{ ok: true, expectFail: null, pageErrors: 0, consoleErrors: 0, stuckToasts: 0, failedRequests: 0 }] };
  assert.equal(rowPasses(good, 1), true);
  assert.equal(rowPasses({ ...good, scenarios: [{ ...good.scenarios[0], skipped: true }] }, 1), false);
  assert.equal(rowPasses({ ...good, scenarios: [{ ...good.scenarios[0], ok: false }] }, 1), false);
  assert.equal(rowPasses({ ...good, scenarios: [{ ...good.scenarios[0], ok: false, expectFail: '#1406' }] }, 1), true);
  assert.equal(rowPasses(good, 2), false);
});
