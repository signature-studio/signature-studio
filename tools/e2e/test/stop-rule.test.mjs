// The acceptance of a scenario and of a row (lib/plan.mjs): a stop of the engine fails a marked scenario too, unless the scenario declares the stop (expectStop);
// a DPR that is not the declared one, and a skipped scenario, fail in any case. The counters rule is unchanged.
import test from 'node:test';
import assert from 'node:assert/strict';
import { needsFailLog, rowPasses, runPasses, scenarioPasses, stopped } from '../lib/plan.mjs';

const base = {
  id: 'settings/language-ja', ok: true, expectFail: null, expectStop: null, pageErrors: 0, consoleErrors: 0, stuckToasts: 0, failedRequests: 0,
  engineStops: 0, engineStuckRunning: false, dprOk: null, skipped: false, error: null,
};
const row = (s) => ({ servedHeadersMatch: true, error: null, scenarios: [s] });
const marked = { ...base, ok: false, expectFail: 'new' };

test('stopped: the engine stopped (a stop report) or stayed in running', () => {
  assert.equal(stopped(base), false);
  assert.equal(stopped({ ...base, engineStops: 1 }), true);
  assert.equal(stopped({ ...base, engineStuckRunning: true }), true);
});

test('a marked scenario that stopped the engine fails its row (the mark does not hide a stop)', () => {
  assert.equal(rowPasses(row(marked), 1), true, 'no stop: a known failure passes');
  assert.equal(rowPasses(row({ ...marked, engineStops: 1 }), 1), false);
  assert.equal(rowPasses(row({ ...marked, engineStuckRunning: true }), 1), false);
});

test('a marked scenario may declare its stop (expectStop: an Issue or new): the row passes', () => {
  assert.equal(rowPasses(row({ ...marked, engineStops: 1, expectStop: '#1382' }), 1), true);
  assert.equal(rowPasses(row({ ...marked, engineStops: 1, expectStop: 'new' }), 1), true);
  assert.equal(rowPasses(row({ ...marked, engineStuckRunning: true, expectStop: '#1382' }), 1), true);
});

test('expectStop allows the stop only: a counter still fails a marked scenario', () => {
  assert.equal(rowPasses(row({ ...marked, engineStops: 1, expectStop: '#1382', pageErrors: 1 }), 1), false);
  assert.equal(rowPasses(row({ ...marked, consoleErrors: 1, expectStop: 'new' }), 1), false);
  assert.equal(rowPasses(row({ ...marked, failedRequests: 1 }), 1), false);
});

test('an unmarked scenario with a stop fails, as before (ok is false for it)', () => {
  assert.equal(scenarioPasses({ ...base, ok: false, engineStops: 1 }), false);
  assert.equal(rowPasses(row({ ...base, ok: false, engineStops: 1, expectStop: '#1382' }), 1), false, 'expectStop needs a mark: the unmarked rule is unchanged');
});

test('a DPR that is not the declared one fails any scenario, marked or not', () => {
  assert.equal(scenarioPasses({ ...base, dprOk: false }), false);
  assert.equal(scenarioPasses({ ...marked, dprOk: false }), false);
  assert.equal(scenarioPasses({ ...base, dprOk: true }), true);
  assert.equal(scenarioPasses({ ...base, dprOk: null }), true, 'not checked (the random display)');
});

test('a skipped scenario fails its row, marked or not', () => {
  assert.equal(scenarioPasses({ ...base, skipped: true }), false);
  assert.equal(scenarioPasses({ ...marked, skipped: true }), false);
});

test('reportOnly does not judge the stops either: the lines are all there', () => {
  assert.equal(runPasses([row({ ...marked, engineStops: 1 })], [1], { reportOnly: true }), true);
  assert.equal(runPasses([row({ ...marked, engineStops: 1 })], [1]), false);
});

test('needsFailLog: a scenario that fails the rule gets the log tail; a passing or skipped one does not', () => {
  assert.equal(needsFailLog({ ...base, ok: false }), true, 'unmarked, not ok');
  assert.equal(needsFailLog(base), false, 'unmarked, ok');
  assert.equal(needsFailLog({ ...marked, engineStops: 1 }), true, 'marked, stopped');
  assert.equal(needsFailLog(marked), false, 'marked, a known failure without a stop');
  assert.equal(needsFailLog({ ...base, skipped: true, ok: false }), false, 'skipped');
});

test('defineScenario: expectStop is a mark (#<Issue> or new) and needs expectFail (a stop is allowed only for a known failure)', async () => {
  const { defineScenario } = await import('../lib/scenario.mjs');
  const run = () => ({ ok: true });
  assert.equal(defineScenario({ id: 'settings/x', expectFail: 'new', expectStop: '#1382', run }).expectStop, '#1382');
  assert.equal(defineScenario({ id: 'settings/x', run }).expectStop, null);
  assert.throws(() => defineScenario({ id: 'settings/x', expectStop: 'new', run }), /expectStop/);
  assert.throws(() => defineScenario({ id: 'settings/x', expectFail: 'new', expectStop: '1382', run }), /expectStop/);
});

test('a legacy scenario declares its stop the same way (expectStop needs expectFail)', async () => {
  const { legacyScenario } = await import('../lib/scenario.mjs');
  const run = () => ({ ok: true });
  assert.equal(legacyScenario({ name: 'x', expectFail: 'new', expectStop: '#1382', run }).expectStop, '#1382');
  assert.equal(legacyScenario({ name: 'x', run }).expectStop, null);
  assert.throws(() => legacyScenario({ name: 'x', expectStop: 'new', run }), /expectStop/);
  assert.throws(() => legacyScenario({ name: 'x', expectFail: 'new', expectStop: '1382', run }), /expectStop/);
});
