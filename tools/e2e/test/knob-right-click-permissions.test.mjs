// The judge of the smoke scenario knob-right-click-permissions (Issue #1482): the menu opens in each of the three permission states, and nothing else happens.
// At a DPR where the knob's place is not calibrated (not 1 or 2: the place was measured at 1 and 2 only) the menu is not required; the counters still are.
import test from 'node:test';
import assert from 'node:assert/strict';
import knob, { judgeKnob, positionNotes } from '../scenarios/smoke/knob-right-click-permissions.mjs';

const state = (extra = {}) => ({ permission: 'prompt', menuShown: true, clipboardReads: 0, visibleStatusRegions: 0, permissionChanges: 0, ...extra });
const judgeWithDenied = (extra) => judgeKnob([state(), state({ permission: 'denied', ...extra }), state({ permission: 'granted' })]);
const noMenu = () => [state({ menuShown: false }), state({ permission: 'denied', menuShown: false }), state({ permission: 'granted', menuShown: false })];

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
  assert.equal(judgeKnob([state()], 2.73), false);
});

test('at DPR 1 and 2 (calibrated) a state without the menu fails, whichever state it is', () => {
  for (const dpr of [1, 2]) {
    assert.equal(judgeKnob(noMenu(), dpr), false, `DPR ${dpr}`);
    assert.equal(judgeKnob([state(), state({ permission: 'denied', menuShown: false }), state({ permission: 'granted' })], dpr), false, `DPR ${dpr}`);
  }
});

test('at DPR 2.73 (not calibrated) no menu in any state passes when the counters are all 0', () => {
  assert.equal(judgeKnob(noMenu(), 2.73), true);
});

test('at DPR 2.73 a state with the menu is judged by its counters, and the states without it need no menu', () => {
  assert.equal(judgeKnob([state(), state({ permission: 'denied', menuShown: false }), state({ permission: 'granted', menuShown: false })], 2.73), true);
});

test('at DPR 2.73 a clipboard read, a visible notice or a permission change fails, with or without the menu', () => {
  assert.equal(judgeKnob(noMenu().map((r, i) => (i === 1 ? { ...r, clipboardReads: 1 } : r)), 2.73), false);
  assert.equal(judgeKnob([state({ visibleStatusRegions: 1, menuShown: false }), state({ permission: 'denied' }), state({ permission: 'granted' })], 2.73), false);
  assert.equal(judgeKnob([state(), state({ permission: 'denied' }), state({ permission: 'granted', permissionChanges: 1, menuShown: false })], 2.73), false);
});

test('the DPR of a run is 1 when none is given: the judge is the calibrated one', () => {
  assert.equal(judgeKnob(noMenu()), false);
});

test('the position note: at DPR 2.73 with no menu in any state, it says the place is unverified and what is judged', () => {
  assert.deepEqual(positionNotes(2.73, noMenu()), ['position-unverified at DPR 2.73: the menu did not open at the calibrated place; the clipboard counters are judged only (reads 0, status 0, permission changes 0)']);
});

test('the position note names the states without the menu when some of them opened it', () => {
  assert.deepEqual(positionNotes(2.73, [state(), state({ permission: 'denied', menuShown: false }), state({ permission: 'granted', menuShown: false })]), [
    'position-unverified at DPR 2.73: the menu did not open at the calibrated place in denied, granted; the clipboard counters are judged only (reads 0, status 0, permission changes 0)',
  ]);
});

test('no position note at DPR 1 and 2, or at 2.73 when the menu opened in every state', () => {
  assert.deepEqual(positionNotes(1, noMenu()), []);
  assert.deepEqual(positionNotes(2, noMenu()), []);
  assert.deepEqual(positionNotes(2.73, [state(), state({ permission: 'denied' }), state({ permission: 'granted' })]), []);
});

test('the knob scenario declares the DPRs its place is calibrated at: 4k-dpr1 and 4k-dpr2', () => {
  assert.deepEqual(knob.displays, ['4k-dpr1', '4k-dpr2']);
});
