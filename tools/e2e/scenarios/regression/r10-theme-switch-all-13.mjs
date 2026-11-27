// R10 (#1408) theme-switch-all-13. Report (L-062, #1382): switching the theme crashes the engine. Expectation: the Settings window's Theme page lists the 13
// themes; each is chosen in turn (its tile is pressed); after each the engine is engine-idle (3 s later) and the picture changes.
// The tiles are pixels of the Theme page's grid (TQuickStringGrid): the window table has no text for them (#1505: the names are drawn in the grid). A tile is found by its
// place on the picture (lib/visual.mjs tilePoint: the grid is scrolled with its scroll bar's thumb). The themes are the .flstheme files of the installer list, in the order of
// the Theme page (themeOrderNames); the engine's log names the file it opens for the chosen tile.
import { themeOrderNames, tilePoint } from '../../lib/visual.mjs';
import { engineLines, pictureHash, settingsPage, sleep, startDefault, table, tops, under, SETTINGS, result } from './common.mjs';

const SCENARIO = {
  name: 'theme-switch-all-13',
  report: 'R10',
  expect: 'Settings > Theme: each of the 13 themes chosen in turn; after each the engine is engine-idle (3 s later) and the picture changes',
};
export const THEMES = themeOrderNames();
const GRID = 'TQuickStringGrid', SCROLLER = 'TQuickScrollBoxScroller', SHEET = 'TVectorSheet';

/** the visible page of the Settings window now: its TVectorSheet (the sheet's place and the table) */
async function sheetNow(guest) {
  const t = await table(guest);
  const st = tops(t).find((w) => w.cls === SETTINGS);
  if (!st) return { page: null, t };
  return { page: under(t, st.lid).find((w) => w.vis && w.cls === SHEET) ?? null, t };
}
const rectOf = (w) => ({ x: w.ax, y: w.ay, w: w.w, h: w.h });
const fileOf = (name) => new RegExp(`[\\\\/]${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\.flstheme`, 'i');

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const sp = await settingsPage(guest, 'Theme');
    if (!sp.page) return result(false, (await guest.state())?.state ?? null, [...start.notes, sp.why ?? 'no Theme page'], { captions: sp.captions });
    const notes = [...start.notes];
    const rows = [];
    for (const [i, name] of THEMES.entries()) {
      const { page, t } = await sheetNow(guest);
      const kids = page && page.text === 'Theme' ? under(t, page.lid) : [];
      const grid = kids.find((w) => w.cls === GRID) ?? null;
      const scroller = kids.find((w) => w.cls === SCROLLER) ?? null;
      const pt = grid && scroller ? await tilePoint(guest, rectOf(grid), rectOf(scroller), i + 1) : null;
      if (!pt) {
        rows.push({ name, found: false });
        notes.push(`theme "${name}": its tile is not in view of the Theme grid`);
        log(notes.at(-1));
        continue;
      }
      await sleep(1000);
      const before = await pictureHash(guest);
      const from = (await engineLines(guest)).length;
      await guest.click(pt.x, pt.y);
      await sleep(3000);
      const st = (await guest.state())?.state ?? null;
      const after = await pictureHash(guest);
      const named = (await engineLines(guest)).slice(from).some((l) => fileOf(name).test(l));
      rows.push({ name, found: true, tile: i + 1, state: st, changed: before !== after, named });
      log(`theme "${name}" (tile ${i + 1}): state ${st}, picture changed ${before !== after}, the engine names its file ${named}`);
    }
    const ok = rows.length === THEMES.length && rows.every((r) => r.found && r.state === 'engine-idle' && r.changed && r.named);
    notes.push(`switches: ${rows.map((r) => `${r.name} ${r.found ? `${r.state}/${r.changed ? 'changed' : 'same'}/${r.named ? 'named' : 'not named'}` : 'not found'}`).join(', ')}`);
    return result(ok, (await guest.state())?.state ?? null, notes, { rows });
  },
};
