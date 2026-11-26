// The judge of the smoke scenario knob-right-click-permissions (Issue #1482): the menu opens in each of the three permission states, and nothing else happens.
import test from 'node:test';
import assert from 'node:assert/strict';
import { judgeKnob } from '../scenarios/smoke/knob-right-click-permissions.mjs';

const state = (extra = {}) => ({ permission: 'prompt', menuShown: true, clipboardReads: 0, visibleStatusRegions: 0, permissionChanges: 0, ...extra });
const judgeWithDenied = (extra) => judgeKnob([state(), state({ permission: 'denied', ...extra }), state({ permission: 'granted' })]);

test('the judge passes when the menu opens in each of the three states and nothing else happens', () => {
  assert.equal(judgeKnob([state(), state({ permission: 'denied' }), state({ permission: 'granted' })]), true);
});

test('a state without the menu fails, whichever state it is', () => {
  assert.equal(judgeKnob([state({ menuShown: false }), state({ permission: 'denied' }), state({ permission: 'granted' })]), false);
  assert.equal(judgeKnob([state(), state({ permission: 'denied', menuShown: false }), state({ permission: 'granted' })]), false);
  assert.equal(judgeKnob([state(), state({ permission: 'denied' }), state({ permission: 'granted', menuShown: false })]), false);
});

test('a clipboard read, a visible notice or a permission change fails the run, in any state', () => {
  assert.equal(judgeWithDenied({ clipboardReads: 1 }), false);
  assert.equal(judgeWithDenied({ visibleStatusRegions: 1 }), false);
  assert.equal(judgeWithDenied({ permissionChanges: 1 }), false);
});

test('a retried right click that opens the menu passes like a first one: the judge reads the state, not the number of clicks', () => {
  assert.equal(judgeWithDenied({ retried: true }), true);
});

test('fewer than the three states fails: nothing is judged on a partial run', () => {
  assert.equal(judgeKnob([state(), state({ permission: 'denied' })]), false);
  assert.equal(judgeKnob([]), false);
});
