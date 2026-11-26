// The window table (lib/dump.mjs) and the picture (lib/picture.mjs), on small texts and pictures made here.
import test from 'node:test';
import assert from 'node:assert/strict';
import { absoluteRect, childrenOfClass, parseDump, rootOf, subtree } from '../lib/dump.mjs';
import { blackTiles, clipRect, decodePicture, regionHash } from '../lib/picture.mjs';

const TABLE = [
  'lid 14 at (0, 0) 3500x2124 style 0x94030000 alpha 0 key 000000 class "TFruityLoopsMainForm" title "FL Studio 2026"',
  'child lid 16 parent lid 14 at (372, 82) 3128x2042 vis 1 class "TWPControl" text ""',
  'child style lid 16 0x54000000',
  'child lid 17 parent lid 16 at (0, 0) 499x246 vis 1 class "TStepSeqForm" text "Channel rack"',
  'child lid 18 parent lid 17 at (5, 30) 488x186 vis 0 class "TVectorPanel" text ""',
  'lid 162 at (4, 36) 208x298 style 0x90000000 alpha 0 key 000000 class "TQuickPopupMenuWindow" title ""',
].join('\n');

test('parseDump reads the top-level and the child windows, and skips the style lines', () => {
  const d = parseDump(TABLE);
  assert.equal(d.top.length, 2);
  assert.deepEqual(d.top[0], { lid: 14, x: 0, y: 0, w: 3500, h: 2124, cls: 'TFruityLoopsMainForm', title: 'FL Studio 2026' });
  assert.equal(d.children.length, 3);
  assert.deepEqual(d.children[1], { lid: 17, parent: 16, x: 0, y: 0, w: 499, h: 246, vis: true, cls: 'TStepSeqForm', text: 'Channel rack' });
  assert.equal(d.children[2].vis, false);
});

test('absoluteRect sums the offsets of the parents; rootOf and subtree follow the parents', () => {
  const d = parseDump(TABLE);
  assert.deepEqual(absoluteRect(d, 17), { x: 372, y: 82, w: 499, h: 246 });
  assert.equal(rootOf(d, 18).lid, 14);
  assert.deepEqual(subtree(d, 14).map((c) => c.lid), [16, 17, 18]);
  assert.deepEqual(childrenOfClass(d, ['TStepSeqForm']).map((c) => c.lid), [17]);
  assert.equal(absoluteRect(d, 999), null);
});

function picture(w, h, px) {
  const stride = w * 4;
  const bytes = Buffer.alloc(stride * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const v = px(x, y);
    const i = y * stride + 4 * x;
    bytes[i] = v[0]; bytes[i + 1] = v[1]; bytes[i + 2] = v[2]; bytes[i + 3] = 255;
  }
  return decodePicture({ base64: bytes.toString('base64'), width: w, height: h });
}

test('regionHash: the same pixels give the same hash, another pixel gives another hash, a rectangle outside gives the empty hash', () => {
  const a = picture(8, 8, () => [10, 20, 30]);
  const b = picture(8, 8, () => [10, 20, 30]);
  const c = picture(8, 8, (x) => (x === 3 ? [1, 2, 3] : [10, 20, 30]));
  assert.equal(regionHash(a, { x: 0, y: 0, w: 8, h: 8 }), regionHash(b, { x: 0, y: 0, w: 8, h: 8 }));
  assert.notEqual(regionHash(a, { x: 0, y: 0, w: 8, h: 8 }), regionHash(c, { x: 0, y: 0, w: 8, h: 8 }));
  assert.equal(regionHash(c, { x: 0, y: 0, w: 3, h: 8 }), regionHash(a, { x: 0, y: 0, w: 3, h: 8 }));
  assert.equal(clipRect(a, { x: 20, y: 20, w: 5, h: 5 }), null);
});

test('blackTiles counts the 64 x 64 tiles whose pixels are all 0 (a partial edge tile counts with its pixels)', () => {
  const p = picture(130, 64, (x) => (x < 64 ? [0, 0, 0] : [5, 0, 0]));
  assert.equal(blackTiles(p, 64), 1);
  const q = picture(130, 64, () => [0, 0, 0]);
  assert.equal(blackTiles(q, 64), 3);
  assert.throws(() => decodePicture({ base64: Buffer.alloc(10).toString('base64'), width: 8, height: 8 }), /picture/);
});
