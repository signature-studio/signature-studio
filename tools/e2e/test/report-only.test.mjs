// The exit of a run with --report-only (run.mjs): the flag takes no value; the rows must all be there and error null; the scenario results do not decide.
import test from 'node:test';
import assert from 'node:assert/strict';
import { parseArgs, rowPasses, runPasses } from '../lib/plan.mjs';

const scenario = (extra = {}) => ({ id: 'input/x', ok: true, expectFail: null, pageErrors: 0, consoleErrors: 0, stuckToasts: 0, failedRequests: 0, ...extra });
const row = (scenarios, extra = {}) => ({ servedHeadersMatch: true, error: null, scenarios, ...extra });

test('--report-only is a flag: it takes no value, so the next argument is read on its own', () => {
  assert.deepEqual(parseArgs(['--area', 'input', '--shard', '1/8', '--report-only']), { area: 'input', shard: '1/8', 'report-only': true });
  assert.deepEqual(parseArgs(['--report-only', '--shard', '1/8']), { 'report-only': true, shard: '1/8' });
});

test('without --report-only, a run passes when every row passes its rule (the rule is unchanged)', () => {
  const good = row([scenario()]);
  assert.equal(runPasses([good], [1]), true);
  assert.equal(runPasses([row([scenario({ ok: false })])], [1]), false);
  assert.equal(runPasses([good, good], [1]), false, 'a row is missing');
  assert.equal(runPasses([good], [1]), rowPasses(good, 1));
});

test('--report-only: a row with a failing scenario passes when its lines are all there and error is null', () => {
  const failing = row([scenario({ ok: false }), scenario({ pageErrors: 2, stuckToasts: 1 })]);
  assert.equal(runPasses([failing], [2], { reportOnly: true }), true);
  assert.equal(runPasses([failing], [2]), false);
  assert.equal(runPasses([row([scenario({ expectFail: 'new', ok: false })]), row([scenario()])], [1, 1], { reportOnly: true }), true);
});

test('--report-only: an error on the row, a missing row, or a missing scenario line fails the run', () => {
  assert.equal(runPasses([row([scenario()], { error: 'row x: boom' })], [1], { reportOnly: true }), false);
  assert.equal(runPasses([row([scenario()]), row([scenario()])], [1], { reportOnly: true }), false, 'a row is missing');
  assert.equal(runPasses([row([scenario()])], [2], { reportOnly: true }), false, 'a scenario line is missing');
  assert.equal(runPasses([], [], { reportOnly: true }), true, 'no row expected and none printed');
});

test('--report-only: the count of the scenario lines is the expected count of the row, and a skipped scenario is still a line', () => {
  assert.equal(runPasses([row([scenario(), scenario({ skipped: true, ok: false })])], [2], { reportOnly: true }), true);
});
