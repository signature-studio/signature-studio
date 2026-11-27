// The page of a shard is gone (its renderer stopped: page.evaluate: Target crashed) and is made again for the next scenario (Issue #1528). A scenario that did
// not run (its page did not start) is an error of its own, and fails the acceptance whether it is marked or not.
import test from 'node:test';
import assert from 'node:assert/strict';
import { erroredScenario, pageGone, scenarioPasses } from '../lib/plan.mjs';

const sc = { id: 'visual/black-dpr-random-2', name: 'visual/black-dpr-random-2', area: 'visual', covers: [], displays: ['4k-dpr1'], expectFail: null, expectStop: null };
const d = { name: '4k-dpr1', dpr: 1, cssWidth: 3840, cssHeight: 2160 };

test('pageGone: the renderer stopped (Target crashed in the error, or the crash event) or the page is closed: the page is made again', () => {
  assert.equal(pageGone({ error: 'Error: evaluate: page.evaluate: Target crashed\n    at Guest.ev' }), true);
  assert.equal(pageGone({ crashed: true }), true);
  assert.equal(pageGone({ closed: true }), true);
  assert.equal(pageGone({ error: 'evaluate: page.evaluate: Target page, context or browser has been closed' }), true);
  assert.equal(pageGone({ error: 'Protocol error (Emulation.setDeviceMetricsOverride): Session closed. Most likely the page has been closed.' }), true);
});

test('pageGone: an error of the scenario itself, or a timeout, leaves the page as it is', () => {
  assert.equal(pageGone({}), false);
  assert.equal(pageGone({ error: null }), false);
  assert.equal(pageGone({ error: 'Error: F10 opened no TMIDIForm (the Settings window) within 15 s' }), false);
  assert.equal(pageGone({ error: 'Error: evaluate: timeout 20000 ms: evaluate' }), false);
});

test('erroredScenario: a scenario that did not run is an error, not ok, and fails the acceptance (marked or not)', () => {
  const r = erroredScenario(sc, 1, d, 'the page did not start', 12);
  assert.equal(r.id, sc.id);
  assert.equal(r.display, '4k-dpr1');
  assert.equal(r.error, 'the page did not start');
  assert.equal(r.ok, false);
  assert.equal(r.ms, 12);
  assert.equal(scenarioPasses(r), false);
  assert.equal(scenarioPasses({ ...r, expectFail: 'new' }), false);
});
