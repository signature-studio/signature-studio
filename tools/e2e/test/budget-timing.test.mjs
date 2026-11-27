// The budget of a display row (lib/plan.mjs budgetClock: one clock per row, the scenario's needsMs), the result's fields (scenarioResult: displays, ms, dprOk,
// expectStop) and the engine log's tail of a failing scenario (tailLines). No browser.
import test from 'node:test';
import assert from 'node:assert/strict';
import { budgetClock, FAIL_LOG_LINES, scenarioResult, tailLines } from '../lib/plan.mjs';
import { defineScenario } from '../lib/scenario.mjs';

test('the budget clock: the remaining time, and whether a scenario that needs some time can start', () => {
  let t = 0;
  const c = budgetClock(60000, () => t);
  assert.equal(c.remainingMs(), 60000);
  assert.equal(c.canStart(), true);
  assert.equal(c.canStart(60000), true);
  assert.equal(c.canStart(60001), false, 'a scenario that needs more than the row has: not started');
  t = 40000;
  assert.equal(c.remainingMs(), 20000);
  assert.equal(c.canStart(30000), false, 'less than 30 s left: not started');
  assert.equal(c.canStart(20000), true);
  t = 60000;
  assert.equal(c.expired(), true);
  assert.equal(c.canStart(), false);
});

test('no budget: never expired, the remaining time is unlimited', () => {
  const c = budgetClock(null, () => 5);
  assert.equal(c.expired(), false);
  assert.equal(c.remainingMs(), Infinity);
  assert.equal(c.canStart(30 * 60000), true);
});

test('each display row has its own clock: the row that used its budget does not take it from the next row', () => {
  let t = 0;
  const now = () => t;
  const first = budgetClock(60000, now);
  t = 60000;
  assert.equal(first.expired(), true);
  const second = budgetClock(60000, now);
  assert.equal(second.expired(), false, 'the second row starts its own clock');
  assert.equal(second.remainingMs(), 60000);
});

test('defineScenario: needsMs is the least time left for the scenario to start (default 0)', () => {
  const s = defineScenario({ id: 'resilience/wait-idle-30min', displays: ['4k-dpr1'], needsMs: 30 * 60000, run: () => ({ ok: true }) });
  assert.equal(s.needsMs, 30 * 60000);
  assert.equal(defineScenario({ id: 'page/x', run: () => ({ ok: true }) }).needsMs, 0);
  assert.throws(() => defineScenario({ id: 'page/x', needsMs: -1, run: () => ({ ok: true }) }), /needsMs/);
});

test('scenarioResult: the declared displays, the time (ms), the DPR check, and the expectStop mark', () => {
  const sc = { id: 'page/crisp-dpr-3', area: 'page', covers: [], displays: ['dpr:3'], expectFail: null, expectStop: null, name: 'page/crisp-dpr-3' };
  const r = scenarioResult(sc, 1, { name: 'dpr:3' });
  assert.deepEqual(r.displays, ['dpr:3']);
  assert.equal(r.ms, 0);
  assert.equal(r.dprOk, null);
  assert.equal(r.expectStop, null);
  assert.equal(r.display, 'dpr:3');
  assert.equal(r.failLog, undefined, 'a result has a failLog only when it fails');
});

test('tailLines: the last 200 lines of the log since the scenario started (all of them when there are fewer)', () => {
  assert.equal(FAIL_LOG_LINES, 200);
  const lines = Array.from({ length: 500 }, (_, i) => `l${i}`);
  assert.deepEqual(tailLines(lines), lines.slice(300));
  assert.deepEqual(tailLines(lines.slice(0, 10)), lines.slice(0, 10));
  assert.deepEqual(tailLines([]), []);
});
