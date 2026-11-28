// R16 (#1408) topbar-menu-value. Report (L-059 family, #1346, #1311; L-070): the top bar's Tempo, Song Position and Pattern Selector open FL's context menu on a
// right click, and choosing an item changes the part's value. Expectation, for each part: the right click opens a menu (a new window that holds the click point),
// the item is chosen, and the part's box in the picture changes (the value is shown differently).
import { changedIn, kOf, newTops, picture, POPUP, sleep, startDefault, table, result } from './common.mjs';

const SCENARIO = {
  name: 'topbar-menu-value',
  report: 'R16',
  expect: 'top bar Tempo, Song Position and Pattern Selector: a right click shows the menu; the chosen item changes the part\'s box in the picture (and no notice stays)',
  // #1406 is closed on main (dd03d8c1: the right click raises no notice that stays); a closed issue is not a mark. A notice that stays is counted as stuckToasts
  expectFail: null,
};
// the parts at DPR 1 (guest px): the click point, the part's box, the menu item (from the menu's top-left corner); the same numbers as tools/topbar-parts
const PARTS = [
  { name: 'Tempo', at: { x: 462, y: 20 }, box: { x0: 437, y0: 8, x1: 491, y1: 33 }, item: { dx: 198, dy: 112 } },
  { name: 'Song Position', at: { x: 566, y: 20 }, box: { x0: 500, y0: 4, x1: 629, y1: 37 }, item: { dx: 84, dy: 46 } },
  { name: 'Pattern Selector', at: { x: 616, y: 58 }, box: { x0: 592, y0: 48, x1: 639, y1: 76 }, item: { dx: 123, dy: 234 } },
];

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const k = await kOf(guest);
    const notes = [...start.notes];
    const rows = [];
    for (const p of PARTS) {
      const before = await picture(guest);
      const lids = (await table(guest)).map((w) => w.lid);
      await guest.rightClick(p.at.x * k, p.at.y * k);
      await sleep(1000);
      const t = await table(guest);
      const menu = newTops(t, lids).find((w) => w.cls === POPUP && p.at.x * k >= w.ax - 4 && p.at.x * k < w.ax + w.w + 4 && p.at.y * k >= w.ay - 4 && p.at.y * k < w.ay + w.h + 4) ?? null;
      if (menu) {
        await guest.click(menu.ax + p.item.dx * k, menu.ay + p.item.dy * k);
        await sleep(1500);
      }
      for (let i = 0; i < 3 && newTops(await table(guest), lids).length > 0; i++) { await guest.escape(); await sleep(800); }
      const after = await picture(guest);
      const box = { x: p.box.x0 * k, y: p.box.y0 * k, w: (p.box.x1 - p.box.x0) * k, h: (p.box.y1 - p.box.y0) * k };
      const changed = changedIn(before, after, box).n;
      rows.push({ name: p.name, menu: menu !== null, changed });
      notes.push(`${p.name}: menu ${menu !== null}, box changed ${changed} px`);
      log(notes.at(-1));
    }
    const ok = rows.every((r) => r.menu && r.changed > 0);
    return result(ok, (await guest.state())?.state ?? null, notes, { rows });
  },
};
