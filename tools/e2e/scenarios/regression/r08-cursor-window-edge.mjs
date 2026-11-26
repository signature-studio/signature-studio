// R08 (#1408) cursor-window-edge. Report (L-061, #1390): over a window's sizing edge the cursor is the sizing arrow, the real one. Expectation: with the mouse
// 3 guest px inside the right edge of a sizable window (WS_THICKFRAME in the window table's style; FL's windows are opened with their keys, one at a time,
// and closed with Escape until one is found), the #desktop cursor is ew-resize; inside its bottom edge, ns-resize. A window without the frame has the arrow by design.
import { sizable, sleep, startDefault, table, result, MAIN } from './common.mjs';

const SCENARIO = {
  name: 'cursor-window-edge',
  report: 'R08',
  expect: 'the first sizable window (WS_THICKFRAME, drawn) among FL\'s own windows (F-keys): the #desktop cursor is ew-resize over its right edge and ns-resize over its bottom edge',
  expectFail: null,
};
const INSIDE = 3;
// the keys that open FL's own windows (tools/cursor: the sweep of F-keys)
const KEYS = [['F5', 116], ['F6', 117], ['F8', 119], ['F9', 120], ['F11', 122], ['F12', 123], ['F2', 113], ['F3', 114], ['F4', 115], ['F1', 112]];

const cursorNow = (guest) => guest.ev("getComputedStyle(document.getElementById('desktop')).cursor");
const hex = (n) => (n === null ? 'none' : `0x${n.toString(16)}`);

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    // the windows FL opens with its own keys are looked at one by one (each is closed with Escape): the first sizable one is the window measured
    let win = null;
    const seenKeys = [];
    for (const [code, vk] of KEYS) {
      await guest.key(code, code, vk);
      await sleep(900);
      const rows = await table(guest);
      const hit = rows.find((w) => w.cls !== MAIN && sizable(w) && w.w > 60 && w.h > 60);
      seenKeys.push(`${code}: ${rows.filter((w) => !w.child && w.vis).map((w) => `"${w.cls}" ${hex(w.style)}`).join(', ')}`);
      if (hit) { win = hit; break; }
      await guest.escape();
      await sleep(500);
    }
    const seen = seenKeys.slice(0, 6);
    if (!win) return result(true, (await guest.state())?.state ?? null, [...start.notes, '枠を持つ窓が無い。FL が決める', `windows: ${seen.join('; ')}`]);
    const e = await guest.pointAt(win.ax + win.w - INSIDE, win.ay + win.h / 2);
    await guest.move(e);
    await sleep(500);
    const east = await cursorNow(guest);
    const s = await guest.pointAt(win.ax + win.w / 2, win.ay + win.h - INSIDE);
    await guest.move(s);
    await sleep(500);
    const south = await cursorNow(guest);
    const ok = east === 'ew-resize' && south === 'ns-resize';
    const notes = [...start.notes, `window "${win.cls}" ${win.child ? 'child' : 'top'} style ${hex(win.style)} at guest (${win.ax}, ${win.ay}) ${win.w}x${win.h}; cursor over the east edge ${east}, over the south edge ${south}`, `windows: ${seen.join('; ')}`];
    log(notes[1]);
    return result(ok, (await guest.state())?.state ?? null, notes, { east, south });
  },
};
