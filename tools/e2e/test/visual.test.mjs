// The pure parts of the visual area (Issue #1496): the cells of a golden, the edge and text readings, the scroll shift, the theme tiles and the cursor reading.
// No browser: the pictures are built here.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CELLS, cellsOf, cellsShare, cursorReading, edgeReadings, scrollShift, sideReading, textReading, tileOfTheme, themeOrderNames, THEMES, LANGUAGES, WINDOWS, FIXED_DPRS } from '../lib/visual.mjs';

/** a picture of w x h with the luminance `value(x, y)` (grey, RGBA) */
function picture(w, h, value) {
  const stride = w * 4;
  const bytes = Buffer.alloc(stride * h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const v = value(x, y);
      const i = y * stride + 4 * x;
      bytes[i] = v; bytes[i + 1] = v; bytes[i + 2] = v; bytes[i + 3] = 255;
    }
  }
  return { w, h, stride, bytes };
}

test('the cells of a flat rectangle are its colour; a moved cell is counted once', () => {
  const flat = picture(64, 64, () => 100);
  const a = cellsOf(flat, { x: 0, y: 0, w: 64, h: 64 });
  assert.equal(a.length, CELLS * CELLS * 3 * 2);
  assert.equal(cellsShare(a, a), 0);
  const moved = picture(64, 64, (x, y) => (x < 4 && y < 4 ? 200 : 100));
  const b = cellsOf(moved, { x: 0, y: 0, w: 64, h: 64 });
  assert.equal(cellsShare(a, b), 1 / (CELLS * CELLS));
});

test('a small colour change (within the cell limit) is not a difference', () => {
  const a = cellsOf(picture(32, 32, () => 100), { x: 0, y: 0, w: 32, h: 32 });
  const b = cellsOf(picture(32, 32, () => 104), { x: 0, y: 0, w: 32, h: 32 });
  assert.equal(cellsShare(a, b), 0);
});

test('a hard edge is sharp; a blurred edge is not; a flat side is not an edge', () => {
  assert.equal(sideReading([0, 0, 0, 0, 0, 0, 200, 200, 200, 200, 200, 200]).sharp, true);
  const blur = sideReading([0, 0, 0, 0, 20, 60, 100, 140, 180, 200, 200, 200]);
  assert.equal(blur.sharp, false);
  assert.equal(sideReading([50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]).sharp, false);
  const faint = sideReading([50, 50, 50, 50, 50, 50, 55, 55, 55, 55, 55, 55]);
  assert.equal(faint.sharp, true);
  assert.equal(faint.contrast, 5);
});

test('a 1 px frame on a window is read as a frame of 1 with sharp sides', () => {
  const rect = { x: 20, y: 20, w: 60, h: 60 };
  const pic = picture(100, 100, (x, y) => {
    const inside = x >= 20 && x < 80 && y >= 20 && y < 80;
    if (!inside) return 0;
    return x === 20 || y === 20 || x === 79 || y === 79 ? 255 : 90;
  });
  const e = edgeReadings(pic, rect);
  assert.equal(e.measured, 4);
  assert.equal(e.width, 1);
  assert.equal(e.sharp, true);
});

test('a window that fills the picture has no outside on its sides', () => {
  const pic = picture(40, 40, () => 90);
  const e = edgeReadings(pic, { x: 0, y: 0, w: 40, h: 40 });
  assert.equal(e.measured, 0);
  assert.equal(e.sharp, false);
});

test('the text reading: a flat rectangle has no text; drawn glyphs have colours and a range', () => {
  assert.deepEqual(textReading(picture(40, 40, () => 60), { x: 0, y: 0, w: 40, h: 40 }).colours, 1);
  const drawn = picture(40, 40, (x) => (x % 4 < 2 ? 0 : 255));
  const r = textReading(drawn, { x: 0, y: 0, w: 40, h: 40 });
  assert.ok(r.colours >= 2);
  assert.equal(r.range, 255);
});

test('the scroll shift finds how far the content moved up between two profiles', () => {
  const a = Array.from({ length: 200 }, (_, i) => Math.sin(i / 7) * 50 + (i % 13) * 3);
  const b = a.slice(30).concat(new Array(30).fill(0));
  assert.equal(scrollShift(a, b, 60, 0.2), 30);
});

test('the Theme page tiles: the default is tile 0 and the 13 themes are tiles 1 to 13 in the order of the list', () => {
  assert.equal(tileOfTheme('(Default)'), 0);
  const order = themeOrderNames();
  assert.equal(order.length, THEMES.length);
  THEMES.forEach(([, name]) => assert.ok(order.includes(name), `${name} is on the Theme page`));
  assert.equal(tileOfTheme(order[0]), 1);
  assert.equal(tileOfTheme(order[12]), 13);
  assert.throws(() => tileOfTheme('No such theme'));
});

test('the cursor reading: an image-set with a declared resolution is read, a named cursor is not a picture', () => {
  const css = 'image-set(url("data:image/png;base64,iVBORw0KGgo=") 2x) 0 0, default';
  const r = cursorReading(css);
  assert.equal(r.isImageSet, true);
  assert.deepEqual(r.scales, [2]);
  assert.equal(r.sameRatio(2), true);
  assert.equal(r.sameRatio(1.5), false);
  assert.equal(cursorReading('ew-resize').isImageSet, false);
});

test('the golden set: 13 themes, 7 languages, 8 windows and 9 ratios', () => {
  assert.equal(THEMES.length, 13);
  assert.equal(Object.keys(LANGUAGES).length, 7);
  assert.equal(WINDOWS.length, 8);
  assert.equal(FIXED_DPRS.length, 7);
});
