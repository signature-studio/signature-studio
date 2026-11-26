// R10 (#1408) theme-switch-all-13. Report (L-062, #1382): switching the theme crashes the engine. Expectation: the Settings window's Theme page lists the 13
// themes; each is chosen in turn (its tile is clicked); after each switch the engine is engine-idle (3 s later) and the picture changes.
import path from 'node:path';
import { clickIn, pictureHash, settingsPage, sleep, startDefault, table, tops, under, SETTINGS, result } from './common.mjs';

const SCENARIO = {
  name: 'theme-switch-all-13',
  report: 'R10',
  expect: 'Settings > Theme: each of the 13 themes chosen in turn; after each the engine is engine-idle (3 s later) and the picture changes',
  // the Theme page's tiles are not in the window table: none can be chosen (#1505; #1382 is closed)
  expectFail: '#1505',
};
export const THEMES = ['Ash', 'Dark orange', 'Dark', 'DEDZII Light', 'DEDZII', 'Light cherry', 'Light pear', 'Light tangerine', 'PPRIIME Dark', 'PPRIIME Light', 'Sytrus', 'Ultra green', 'Veela'];

/** the visible page of the Settings window now: its TVectorSheet (the sheet's place and the table) */
async function sheetNow(guest) {
  const t = await table(guest);
  const st = tops(t).find((w) => w.cls === SETTINGS);
  if (!st) return { page: null, t };
  return { page: under(t, st.lid).find((w) => w.vis && w.cls === 'TVectorSheet') ?? null, t };
}

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const sp = await settingsPage(guest, 'Theme');
    if (!sp.page) return result(false, (await guest.state())?.state ?? null, [...start.notes, sp.why ?? 'no Theme page'], { captions: sp.captions });
    const notes = [...start.notes];
    const rows = [];
    for (const name of THEMES) {
      let tile = null;
      for (let tries = 0; tries < 8 && !tile; tries++) {
        const { page, t } = await sheetNow(guest);
        if (page) tile = under(t, page.lid).find((w) => w.text === name && w.vis && w.w > 0) ?? null;
        if (!tile) {
          const p = page ?? sp.page;
          await guest.wheel(p.ax + p.w / 2, p.ay + p.h / 2, 2, 120);
          await sleep(400);
        }
      }
      if (!tile) {
        if (guest.shots) await guest.shot(path.join(guest.shots, 'theme-page.png'));
        const { page, t } = await sheetNow(guest);
        const texts = page ? under(t, page.lid).map((w) => w.text).filter(Boolean).slice(0, 40) : [];
        rows.push({ name, found: false });
        notes.push(`theme "${name}": no tile in the Theme page (captions there: ${JSON.stringify(texts)})`);
        break;
      }
      const before = await pictureHash(guest);
      await clickIn(guest, tile, 0.5, 0.5);
      await sleep(3000);
      const st = (await guest.state())?.state ?? null;
      const after = await pictureHash(guest);
      rows.push({ name, found: true, state: st, changed: before !== after });
      log(`theme "${name}": state ${st}, picture changed ${before !== after}`);
    }
    const ok = rows.length === THEMES.length && rows.every((r) => r.found && r.state === 'engine-idle' && r.changed);
    notes.push(`switches: ${rows.map((r) => `${r.name} ${r.found ? `${r.state}/${r.changed ? 'changed' : 'same'}` : 'not found'}`).join(', ')}`);
    return result(ok, (await guest.state())?.state ?? null, notes, { rows });
  },
};
