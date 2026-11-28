// The rows of a run (lib/plan.mjs): a display's row holds the scenarios that declare that display (--displays only names the rows), the DPR of a row and of the page.
import test from 'node:test';
import assert from 'node:assert/strict';
import { dprMatches, fixedDpr, rowsFor } from '../lib/plan.mjs';

const item = (id, displays) => ({ sc: { id, displays } });
const ids = (rows) => rows.map((r) => r.items.map((i) => i.sc.id));

test('rowsFor: a row holds the scenarios that declare its display, and no others', () => {
  const items = [item('page/a', ['4k-dpr1', '4k-random']), item('page/b', ['dpr:2']), item('page/c', ['4k-dpr1'])];
  assert.deepEqual(ids(rowsFor('4k-dpr1', items)), [['page/a', 'page/c']]);
  assert.deepEqual(ids(rowsFor('dpr:2', items)), [['page/b']]);
  assert.deepEqual(ids(rowsFor('4k-dpr2', items)), [[]]);
});

test('rowsFor: --displays does not widen a scenario; a dpr:N scenario is not run in the 4k-dpr1 row', () => {
  const crisp = item('page/crisp-dpr-3', ['dpr:3']);
  assert.deepEqual(ids(rowsFor('4k-dpr1', [crisp])), [[]]);
  assert.deepEqual(ids(rowsFor('4k-dpr2', [crisp])), [[]]);
  assert.deepEqual(ids(rowsFor('dpr:3', [crisp])), [['page/crisp-dpr-3']]);
});

test('rowsFor: the random display gives two rows, each with the scenarios that declare it', () => {
  const items = [item('page/r', ['4k-random']), item('page/s', ['4k-dpr1'])];
  const rows = rowsFor('4k-random', items);
  assert.equal(rows.length, 2);
  assert.deepEqual(ids(rows), [['page/r'], ['page/r']]);
});

test('fixedDpr: the DPR of a display with one fixed DPR; null for the random one', () => {
  assert.equal(fixedDpr('dpr:3'), 3);
  assert.equal(fixedDpr('dpr:1.25'), 1.25);
  assert.equal(fixedDpr('4k-dpr1'), 1);
  assert.equal(fixedDpr('4k-dpr2'), 2);
  assert.equal(fixedDpr('4k-random'), null);
});

test('dprMatches: the page shows the row DPR (true), another DPR (false), or the random row is not checked (null)', () => {
  assert.equal(dprMatches('dpr:3', 3), true);
  assert.equal(dprMatches('dpr:3', 2), false);
  assert.equal(dprMatches('dpr:1.25', 1.25), true);
  assert.equal(dprMatches('4k-random', 1.37), null);
});
