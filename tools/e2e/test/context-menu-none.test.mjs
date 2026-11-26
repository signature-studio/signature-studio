// The judge of the smoke scenario context-menu-none (Issue #1482): every contextmenu event of a region is cancelled by the page; the presses are not judged.
import test from 'node:test';
import assert from 'node:assert/strict';
import { judge } from '../scenarios/smoke/context-menu-none.mjs';

const real = (extra = {}) => ({ region: 'canvas', kind: 'real', events: 18, prevented: 18, presses: 18, pressesCancelled: 18, editable: 0, inFrame: 0, tags: {}, focus: {}, ...extra });
const synthetic = (extra = {}) => ({ region: 'toolbar', kind: 'synthetic', events: 20, prevented: 20, presses: 0, pressesCancelled: 0, editable: 0, inFrame: 0, tags: {}, focus: {}, ...extra });

test('a real region passes when its contextmenu events are all cancelled, whatever the presses that reached the page (none counted: still passes)', () => {
  assert.equal(judge([real({ region: 'before-idle', presses: 0, pressesCancelled: 0 }), real({ presses: 0, pressesCancelled: 0 }), real({ region: 'desktop', presses: 0 }), synthetic(), synthetic({ region: 'sidebar' })]), true);
  assert.equal(judge([real({ presses: 20, pressesCancelled: 3 }), synthetic()]), true);
});

test('a region with no contextmenu event fails, real or synthetic', () => {
  assert.equal(judge([real({ events: 0, prevented: 0 })]), false);
  assert.equal(judge([synthetic({ events: 0, prevented: 0 })]), false);
});

test('a contextmenu the page did not cancel fails: prevented must equal events', () => {
  assert.equal(judge([real({ events: 18, prevented: 17 })]), false);
  assert.equal(judge([real({ events: 18, prevented: 0 })]), false);
  assert.equal(judge([synthetic({ events: 20, prevented: 19 })]), false);
});

test('a synthetic region passes only when all N (20) of its events came', () => {
  assert.equal(judge([synthetic({ events: 19, prevented: 19 })]), false);
  assert.equal(judge([synthetic({ events: 21, prevented: 21 })]), false);
  assert.equal(judge([synthetic({ events: 20, prevented: 20 })]), true);
});

test('a contextmenu whose target is in a frame fails', () => {
  assert.equal(judge([real({ inFrame: 1 })]), false);
  assert.equal(judge([synthetic({ inFrame: 2 })]), false);
});

test('an editable target is recorded, not judged: a paste in an editable place is what the browser offers there', () => {
  assert.equal(judge([real({ editable: 5 })]), true);
  assert.equal(judge([synthetic({ editable: 20 })]), true);
});
