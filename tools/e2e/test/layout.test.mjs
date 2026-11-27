// The places of a window's parts as ratios of the window (Issue #1507): a part's ratio is its DPR 1 distance from the window's top-left over the window's size at DPR 1.
// The rectangles are the window table's, measured at DPR 1 and at DPR 2 (the same window, the same picture of 3840 x 2160). The same ratio must place the same part at both.
import test from 'node:test';
import assert from 'node:assert/strict';
import { box, boxH, place, placeH } from '../lib/layout.mjs';
import { RACK_LAYOUT, rackPoint, rowStepsRect } from '../lib/windows.mjs';
import { TAB_CLICK, inPanel, SETTINGS_W1 } from '../lib/settings.mjs';

const RACK_1 = { x: 372, y: 82, w: 499, h: 246 };
const RACK_2 = { x: 747, y: 164, w: 998, h: 492 };
const MIXER_1 = { x: 388, y: 1628, w: 3436, h: 500 };
const MIXER_2 = { x: 779, y: 1096, w: 3029, h: 1000 };
const SELECTOR_1 = { x: 1591, y: 663, w: 658, h: 32 };
const SELECTOR_2 = { x: 1263, y: 246, w: 1314, h: 64 };
const SETTINGS_1 = { x: 1586, y: 630, w: 668, h: 900 };
const SETTINGS_2 = { x: 1252, y: 180, w: 1336, h: 1800 };
const near = (a, b, tol, msg) => assert.ok(Math.abs(a - b) <= tol, `${msg}: ${a} vs ${b} (tolerance ${tol})`);

test('place: the ratios of the width (x) and the height (y) from the top-left', () => {
  assert.deepEqual(place({ x: 10, y: 20, w: 200, h: 100 }, 0.5, 0.25), { x: 110, y: 45 });
  assert.deepEqual(place({ x: -4, y: 0, w: 50, h: 40 }, 0, 1), { x: -4, y: 40 });
});

test('box: the top-left by the ratios, the size by the ratios of the width and the height', () => {
  assert.deepEqual(box({ x: 10, y: 20, w: 200, h: 100 }, 0.1, 0.2, 0.5, 0.5), { x: 30, y: 40, w: 100, h: 50 });
});

test('placeH and boxH: the ratios of the height only (the Mixer: its width is cut by the screen at DPR 2)', () => {
  assert.deepEqual(placeH({ x: 10, y: 20, w: 9000, h: 100 }, 0.5, 0.25), { x: 60, y: 45 });
  assert.deepEqual(boxH({ x: 10, y: 20, w: 9000, h: 100 }, 0.1, 0.2, 0.5, 0.5), { x: 20, y: 40, w: 50, h: 50 });
});

test('the same ratio of the rack places the same part at DPR 1 and DPR 2 (the rack is 2 times larger at DPR 2)', () => {
  for (const [row, col] of [[0, 0], [3, 7], [7, 15]]) {
    const p1 = rackPoint(RACK_1, row, col), p2 = rackPoint(RACK_2, row, col);
    near(p2.x - RACK_2.x, 2 * (p1.x - RACK_1.x), 1e-6, `step ${row}.${col} x`);
    near(p2.y - RACK_2.y, 2 * (p1.y - RACK_1.y), 1e-6, `step ${row}.${col} y`);
  }
  near(rackPoint(RACK_1, 0, 0).x - RACK_1.x, 219 + 0.5 * 15.5, 1e-9, 'step 1.1 at DPR 1');
});

test('the row steps area of the rack is the same part at both DPRs (the rectangle scales with the rack)', () => {
  const a = rowStepsRect(RACK_1, 2), b = rowStepsRect(RACK_2, 2);
  near(b.x - RACK_2.x, 2 * (a.x - RACK_1.x), 1e-6, 'x');
  near(b.y - RACK_2.y, 2 * (a.y - RACK_1.y), 1e-6, 'y');
  near(b.w, 2 * a.w, 1e-6, 'w');
  near(b.h, 2 * a.h, 1e-6, 'h');
});

test('the rack offsets at DPR 1 are the guest px measured there (219 px to the first step of a 499-px rack)', () => {
  near(RACK_LAYOUT.stepX0 * 499, 219, 1e-9, 'stepX0');
  near(RACK_LAYOUT.rowY0 * 246, 62, 1e-9, 'rowY0');
});

test('the Mixer: its slots are 127 px from its left and 44 px apart at DPR 1 (500 px high), and 2 times that at DPR 2 (1000 px high)', () => {
  const s1 = placeH(MIXER_1, 127 / 500, 120 / 500), s2 = placeH(MIXER_2, 127 / 500, 120 / 500);
  near(s2.x - MIXER_2.x, 2 * (s1.x - MIXER_1.x), 1e-6, 'slot 1 x');
  near(s2.y - MIXER_2.y, 2 * (s1.y - MIXER_1.y), 1e-6, 'slot 1 y');
  const k1 = placeH(MIXER_1, (127 + 44 * 7) / 500, 0), k2 = placeH(MIXER_2, (127 + 44 * 7) / 500, 0);
  near(k2.x - MIXER_2.x, 2 * (k1.x - MIXER_1.x), 1e-6, 'slot 8 x');
});

test('the Settings tabs: the click of a tab is its ratio of the selector (658 px wide at DPR 1, 1314 at DPR 2)', () => {
  for (const name of ['MIDI', 'General', 'Debug', 'About']) {
    const c1 = place(SELECTOR_1, TAB_CLICK[name], 0.5), c2 = place(SELECTOR_2, TAB_CLICK[name], 0.5);
    near(c2.x - SELECTOR_2.x, (c1.x - SELECTOR_1.x) * (SELECTOR_2.w / SELECTOR_1.w), 1e-6, `${name} x`);
    near(c2.y - SELECTOR_2.y, SELECTOR_2.h / 2, 1e-6, `${name} y`);
  }
  near(TAB_CLICK.General * SELECTOR_1.w, 114 + 8, 1e-9, 'General at DPR 1');
});

test('the Settings panel offsets: the DPR 1 offset over the settings window (668 x 900) scaled to the window now', () => {
  assert.deepEqual(SETTINGS_W1, { w: 668, h: 900 });
  const panel1 = { x: 1616, y: 703, w: 608, h: 280 }, panel2 = { x: 1312, y: 326, w: 1216, h: 561 };
  const p1 = inPanel(panel1, SETTINGS_1, 40, 124), p2 = inPanel(panel2, SETTINGS_2, 40, 124);
  near(p2.x - panel2.x, 2 * (p1.x - panel1.x), 1e-6, 'x');
  near(p2.y - panel2.y, 2 * (p1.y - panel1.y), 1e-6, 'y');
});
