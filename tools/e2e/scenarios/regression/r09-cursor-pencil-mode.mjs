// R09 (#1408) cursor-pencil-mode. Report (L-061, #1392): in the Piano roll (its Pencil mode, the default tool) the cursor over the note grid is FL's own pencil
// picture, not the page's standard arrow. Expectation: the #desktop cursor is a picture (url(...)) over the grid of the Piano roll opened with F7.
import { PIANO_ROLL, sleep, startDefault, waitRow, result } from './common.mjs';

const SCENARIO = {
  name: 'cursor-pencil-mode',
  report: 'R09',
  expect: 'the Piano roll (F7) in its Pencil mode: over the note grid the #desktop cursor is a picture (url(...)) (the picture is not yet matched with FL\'s pencil resource)',
  expectFail: null,
};

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    await guest.key('F7', 'F7', 118);
    const pr = await waitRow(guest, (w) => w.cls === PIANO_ROLL && /^Piano roll/.test(w.text), 15000);
    if (!pr.row) return result(false, (await guest.state())?.state ?? null, [...start.notes, 'no Piano roll window after F7']);
    const w = pr.row;
    const c = await guest.pointAt(w.ax + w.w * 0.5, w.ay + w.h * 0.6);
    await guest.move(c);
    await sleep(400);
    const css = await guest.ev("getComputedStyle(document.getElementById('desktop')).cursor");
    const picture = await guest.ev("window.__app.cursor ? window.__app.cursor.css : null");
    const ok = typeof css === 'string' && /url\(/.test(css);
    const notes = [...start.notes, `cursor over the Piano roll's grid: ${String(css).slice(0, 120)} (the page's cursor state: ${String(picture).slice(0, 40)})`];
    log(notes.at(-1));
    return result(ok, (await guest.state())?.state ?? null, notes, { css: String(css).slice(0, 200) });
  },
};
