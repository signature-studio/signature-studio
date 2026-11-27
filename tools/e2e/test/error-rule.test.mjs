// The acceptance of a scenario whose run threw (error is not null: e.g. page.evaluate: Target crashed, the page is dead). Such a result is not accepted,
// marked or not: a mark (a known failure) does not hide an error (Issue #1528). The row (rowPasses), the run's exit and deploy.yml's judge (judge.jq) say the same.
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { needsFailLog, rowPasses, runPasses, scenarioPasses } from '../lib/plan.mjs';

const base = {
  id: 'visual/black-dpr-random-2', ok: true, expectFail: null, expectStop: null, pageErrors: 0, consoleErrors: 0, stuckToasts: 0, failedRequests: 0,
  engineStops: 0, engineStuckRunning: false, dprOk: null, skipped: false, error: null,
};
const row = (s) => ({ servedHeadersMatch: true, error: null, scenarios: [s] });
const CRASH = 'page.evaluate: Target crashed';
const marked = { ...base, ok: false, expectFail: 'new' };
const crashed = { ...base, ok: false, error: CRASH };

test('an error fails an unmarked scenario, and its row', () => {
  assert.equal(scenarioPasses(crashed), false);
  assert.equal(rowPasses(row(crashed), 1), false);
});

test('an error fails a marked scenario too: a mark does not hide the error (the 15 `new` results of the Target crashed run)', () => {
  assert.equal(scenarioPasses({ ...marked, error: CRASH }), false);
  assert.equal(scenarioPasses({ ...marked, expectFail: '#1528', error: CRASH }), false);
  assert.equal(scenarioPasses({ ...marked, expectFail: '#1528', expectStop: 'new', engineStops: 1, error: CRASH }), false);
  assert.equal(rowPasses(row({ ...marked, error: CRASH }), 1), false);
  assert.equal(runPasses([row({ ...marked, error: CRASH })], [1]), false);
});

test('an error with no counters still fails a marked scenario (the counters are all 0)', () => {
  assert.equal(scenarioPasses({ ...marked, pageErrors: 0, consoleErrors: 0, failedRequests: 0, error: CRASH }), false);
});

test('a result with no error is accepted as before (marked or not)', () => {
  assert.equal(scenarioPasses(base), true);
  assert.equal(scenarioPasses(marked), true);
  assert.equal(rowPasses(row(marked), 1), true);
});

test('needsFailLog: an error gets the log tail, marked or not', () => {
  assert.equal(needsFailLog({ ...crashed }), true);
  assert.equal(needsFailLog({ ...marked, error: CRASH }), true);
  assert.equal(needsFailLog({ ...marked, skipped: true, error: CRASH }), false, 'skipped: not run');
});

test('reportOnly keeps its rule: the scenario results do not count (the caller reads the errors)', () => {
  assert.equal(runPasses([row({ ...marked, error: CRASH })], [1], { reportOnly: true }), true);
});

const here = path.dirname(fileURLToPath(import.meta.url));
const JUDGE = path.join(here, '..', 'judge.jq');
const hasJq = spawnSync('jq', ['--version']).status === 0;
const judged = (rows) => spawnSync('jq', ['-e', '-s', '-f', JUDGE], { input: rows.map((r) => JSON.stringify({ ...r, display: { achievedFps: 120 } })).join('\n'), encoding: 'utf8' });

test('judge.jq (deploy.yml): a marked scenario with an error fails; without the error it passes', { skip: !hasJq && 'jq is not installed' }, () => {
  assert.notEqual(judged([row({ ...marked, error: CRASH })]).status, 0, 'marked + error: fails');
  assert.notEqual(judged([row(crashed)]).status, 0, 'unmarked + error: fails');
  assert.equal(judged([row(marked)]).status, 0, 'marked, no error: passes');
  assert.equal(judged([row(base)]).status, 0, 'unmarked, ok: passes');
});
