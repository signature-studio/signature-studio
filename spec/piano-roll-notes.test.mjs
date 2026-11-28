#!/usr/bin/env node
// spec/piano-roll-notes.test.mjs : tools/piano-roll-tools/measure.mjs の数え方(countNotes: 後にだけ現れた色の連結成分のうち、幅・高さ 4 px 以上の物)と、窓の一色の割合(dominantShare)を、合成の画面で確かめる。
// 判定の式は受け入れと同じ(notes が 1 以上は通り、0 は落ちる)。合成の画面だけを使い、実機の画面は使わない。終了コード 0 = 全部通った。
import { areaHash, countNotes, dominantShare } from '../tools/piano-roll-tools/measure.mjs';

const W = 60, H = 40;
const BG = 0x102030ff, NOTE = 0x40c080ff, OTHER = 0xffff00ff;

function picture(boxes, bg = BG) {
  const b = Buffer.alloc(W * H * 4);
  for (let i = 0; i < W * H; i++) b.writeUInt32BE(bg, i * 4);
  for (const { x, y, w, h, c } of boxes) for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) b.writeUInt32BE(c, (yy * W + xx) * 4);
  return { width: W, height: H, base64: b.toString('base64') };
}
const ALL = { x: 0, y: 0, w: W, h: H };
let failed = 0;
const check = (name, got, want) => {
  const ok = JSON.stringify(got) === JSON.stringify(want);
  if (!ok) failed++;
  console.log(`${ok ? 'ok ' : 'NG '} ${name}: ${JSON.stringify(got)}${ok ? '' : ` (want ${JSON.stringify(want)})`}`);
};
const judge = (notes) => (notes >= 1 ? 'pass' : 'fail');

{
  const r = countNotes(picture([]), picture([{ x: 5, y: 5, w: 10, h: 6, c: NOTE }]), ALL);
  check('one block 10x6 that is new (notes)', r.notes, 1);
  check('one block: the rectangle', r.noteRects, [{ x: 5, y: 5, w: 10, h: 6 }]);
  check('one block: the note colour, measured', r.noteColour, '#40c080');
  check('one block: the judgement', judge(r.notes), 'pass');
}
{
  const after = picture([
    { x: 5, y: 5, w: 10, h: 6, c: NOTE }, { x: 30, y: 20, w: 4, h: 4, c: NOTE },
    { x: 50, y: 30, w: 3, h: 3, c: NOTE }, { x: 20, y: 35, w: 10, h: 3, c: NOTE },
  ]);
  const r = countNotes(picture([]), after, ALL);
  check('two blocks; a 3x3 dot and a 10x3 bar are left out (notes)', r.notes, 2);
  check('two blocks: the rectangles', r.noteRects, [{ x: 5, y: 5, w: 10, h: 6 }, { x: 30, y: 20, w: 4, h: 4 }]);
}
{
  const r = countNotes(picture([]), picture([{ x: 0, y: 0, w: 4, h: 4, c: NOTE }, { x: 4, y: 4, w: 4, h: 4, c: NOTE }]), ALL);
  check('two blocks that touch only at a corner are two parts (notes)', r.notes, 2);
}
{
  const same = picture([{ x: 5, y: 5, w: 10, h: 6, c: NOTE }]);
  const r = countNotes(same, picture([{ x: 5, y: 5, w: 10, h: 6, c: NOTE }]), ALL);
  check('no colour is new: notes 0', r.notes, 0);
  check('no colour is new: noteColour null', r.noteColour, null);
  check('no colour is new: the judgement', judge(r.notes), 'fail');
}
{
  // the most common new colour decides: yellow (60 px) is the note colour; the 3x9 green block (27 px) is not counted
  const r = countNotes(picture([]), picture([{ x: 5, y: 5, w: 10, h: 6, c: OTHER }, { x: 20, y: 5, w: 3, h: 9, c: NOTE }]), ALL);
  check('the most common new colour is the note colour: yellow 10x6 counted, the green 3x9 is not', [r.notes, r.noteColour], [1, '#ffff00']);
}
{
  // the block is cut to 3x3 by the rectangle: too small
  const r = countNotes(picture([]), picture([{ x: 5, y: 5, w: 10, h: 6, c: NOTE }]), { x: 0, y: 0, w: 8, h: 8 });
  check('a block cut by the rectangle to 3x3 is not a note: notes 0', r.notes, 0);
}
{
  check('dominantShare: a uniform rectangle covers 100%', dominantShare(picture([]), ALL).share, 1);
  const dot = dominantShare(picture([{ x: 0, y: 0, w: 3, h: 3, c: NOTE }]), ALL);
  check('dominantShare: a 3x3 dot: the background covers at least 95% (not painted)', dot.share >= 0.95, true);
  const block = dominantShare(picture([{ x: 0, y: 0, w: 12, h: 40, c: NOTE }]), ALL);
  check('dominantShare: a 12x40 block: the background covers 80% (painted)', [block.share, block.colour === BG], [0.8, true]);
}

{
  const a = picture([{ x: 5, y: 5, w: 4, h: 4, c: NOTE }]);
  check('areaHash: the same pixels give the same hash', areaHash(a, ALL) === areaHash(picture([{ x: 5, y: 5, w: 4, h: 4, c: NOTE }]), ALL), true);
  check('areaHash: a changed pixel changes the hash', areaHash(a, ALL) !== areaHash(picture([{ x: 5, y: 5, w: 4, h: 4, c: OTHER }]), ALL), true);
  check('areaHash: a rectangle outside the picture has no hash (null)', areaHash(a, { x: 100, y: 100, w: 4, h: 4 }), null);
}

if (failed) console.log(`${failed} check(s) failed`);
else console.log('all checks passed');
process.exitCode = failed ? 1 : 0;
