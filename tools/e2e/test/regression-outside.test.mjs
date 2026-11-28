// The outside-of-window check of R13 (Issue #1433): the changed pixels outside the window and the excluded rectangles, and the top bar's CPU panel in the window table.
import test from 'node:test';
import assert from 'node:assert/strict';
import { changedOutside, parseTable, topbarMeasuredRect } from '../scenarios/regression/common.mjs';

const picture = (w, h, fill) => {
  const stride = w * 4;
  const bytes = Buffer.alloc(stride * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) bytes.set([fill(x, y), 0, 0, 255], y * stride + 4 * x);
  return { w, h, stride, bytes };
};

test('changedOutside counts the changed pixels outside the rectangle and the excluded ones, and gives their box', () => {
  const a = picture(10, 10, () => 0);
  const b = picture(10, 10, (x, y) => (x === 1 && y === 1 ? 9 : x === 8 && y === 7 ? 9 : x === 3 && y === 2 ? 9 : 0));
  assert.deepEqual(changedOutside(a, b, { x: 0, y: 0, w: 2, h: 2 }), { n: 2, box: { x: 3, y: 2, w: 6, h: 6 } });
  assert.deepEqual(changedOutside(a, b, { x: 0, y: 0, w: 2, h: 2 }, [{ x: 7, y: 6, w: 3, h: 3 }]), { n: 1, box: { x: 3, y: 2, w: 1, h: 1 } });
  assert.deepEqual(changedOutside(a, b, { x: 0, y: 0, w: 10, h: 10 }), { n: 0, box: null });
});

test('changedOutside does not exclude a pixel that is only next to an excluded rectangle', () => {
  const a = picture(6, 1, () => 0);
  const b = picture(6, 1, (x) => (x === 4 ? 1 : 0));
  assert.equal(changedOutside(a, b, { x: 0, y: 0, w: 0, h: 0 }, [{ x: 0, y: 0, w: 4, h: 1 }]).n, 1);
  assert.equal(changedOutside(a, b, { x: 0, y: 0, w: 0, h: 0 }, [{ x: 0, y: 0, w: 4, h: 1 }]).box.x, 4);
});

const TOP = [
  'lid 14 at (0, 0) 3840x2160 style 0x94030000 alpha 0 key 000000 class "TFruityLoopsMainForm" title "FL Studio 2026"',
  'child lid 50 parent lid 14 at (0, 0) 3840x82 vis 1 class "TQuickControlBar" text ""',
  'child lid 51 parent lid 50 at (0, 0) 3839x41 vis 1 class "TQuickEditToolBar" text ""',
  'child lid 52 parent lid 51 at (861, 0) 172x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 71 parent lid 50 at (0, 41) 3839x41 vis 1 class "TQuickEditToolBar" text ""',
  'child lid 72 parent lid 71 at (935, 0) 172x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 73 parent lid 71 at (532, 0) 92x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 74 parent lid 71 at (900, 0) 35x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 75 parent lid 71 at (1, 0) 300x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 76 parent lid 71 at (624, 0) 144x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 77 parent lid 51 at (1033, 0) 156x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 78 parent lid 51 at (301, 0) 196x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 79 parent lid 51 at (497, 0) 136x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 80 parent lid 51 at (1, 0) 300x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 82 parent lid 51 at (3771, 0) 68x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 83 parent lid 51 at (765, 0) 96x40 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 84 parent lid 51 at (1191, 0) 2580x41 vis 1 class "TQuickEditToolbarPanel" text ""',
  'child lid 85 parent lid 51 at (900, 0) 35x41 vis 0 class "TQuickEditToolbarPanel" text ""',
].join('\n');

test('topbarMeasuredRect is the CPU panel: the shown panel of the first row that ends where the splitter (the 13th shown panel) begins', () => {
  const rows = parseTable(TOP);
  assert.deepEqual(topbarMeasuredRect(rows), { x: 1033, y: 0, w: 156, h: 41 });
});

test('topbarMeasuredRect is null when the splitter is not in the table, and the rectangle moves with the table (the same at any DPR)', () => {
  assert.equal(topbarMeasuredRect(parseTable(TOP.split('\n').slice(0, 6).join('\n'))), null);
  const twice = TOP.replace(/at \((\d+), (\d+)\) (\d+)x(\d+)/g, (m, x, y, w, h) => `at (${2 * x}, ${2 * y}) ${2 * w}x${2 * h}`);
  assert.deepEqual(topbarMeasuredRect(parseTable(twice)), { x: 2066, y: 0, w: 312, h: 82 });
});
