// R13 (#1408) settings-general-scroll. Report (L-064, #1384): the Settings window's General page keeps its drawing when it is scrolled. Expectation: the
// General page (F10) is scrolled down and up by the wheel (6 notches each); the pictures before and after each scroll differ only inside the Settings
// window (no change outside its rectangle), and the window is not blank (under 2% of its pixels are black at the bottom of the scroll).
import { blankShare, changedOutside, picture, settingsPage, sleep, startDefault, result } from './common.mjs';

const SCENARIO = {
  name: 'settings-general-scroll',
  report: 'R13',
  expect: 'Settings > General scrolled down and up (6 notches each): no picture change outside the Settings window; under 2% of its pixels black',
  // at DPR 1 the scroll changes the picture outside the window (#1433; #1384 is closed)
  expectFail: '#1433',
};
const NOTCHES = 6;

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const sp = await settingsPage(guest, 'General');
    if (!sp.page || !sp.st) return result(false, (await guest.state())?.state ?? null, [...start.notes, sp.why ?? 'no General page']);
    await sleep(800);
    const st = sp.st;
    const rect = { x: st.ax, y: st.ay, w: st.w, h: st.h };
    const top = await picture(guest);
    const cx = st.ax + st.w / 2, cy = st.ay + st.h / 2;
    await guest.wheel(cx, cy, NOTCHES, 120);
    await sleep(900);
    const down = await picture(guest);
    await guest.wheel(cx, cy, NOTCHES, -120);
    await sleep(900);
    const up = await picture(guest);
    const outDown = changedOutside(top, down, rect);
    const outUp = changedOutside(down, up, rect);
    const blank = blankShare(down, rect);
    const ok = outDown === 0 && outUp === 0 && blank < 0.02;
    const notes = [...start.notes, `Settings at guest (${st.ax}, ${st.ay}) ${st.w}x${st.h}; changed outside it: down ${outDown} px, up ${outUp} px; black share ${(blank * 100).toFixed(2)} %`];
    log(notes.at(-1));
    return result(ok, (await guest.state())?.state ?? null, notes, { outDown, outUp, blank: Number(blank.toFixed(4)) });
  },
};
