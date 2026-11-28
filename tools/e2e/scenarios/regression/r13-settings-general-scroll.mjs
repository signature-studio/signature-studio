// R13 (#1408) settings-general-scroll. Report (L-064, #1384): the Settings window's General page keeps its drawing when it is scrolled. Expectation: the
// General page (F10) is scrolled down and up by the wheel (6 notches each); the pictures before and after each scroll differ only inside the Settings
// window (no change outside its rectangle), and the window is not blank (under 2% of its pixels are black at the bottom of the scroll).
import { blankShare, changedIn, changedOutside, picture, settingsPage, sleep, startDefault, result, topbarMeasuredRect } from './common.mjs';

const SCENARIO = {
  name: 'settings-general-scroll',
  report: 'R13',
  expect: 'Settings > General scrolled down and up (6 notches each): no picture change outside the Settings window; under 2% of its pixels black',
  expectFail: null,
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
    const cx = st.ax + st.w / 2, cy = st.ay + st.h / 2;
    // the pointer is over the window's centre before the wheel, as a user's is: a wheel that jumps the pointer there is a pointer move (the top bar's hint follows it, #1433)
    await guest.move(await guest.pointAt(cx, cy));
    await sleep(1000);
    const top = await picture(guest);
    await guest.wheel(cx, cy, NOTCHES, 120);
    await sleep(900);
    const down = await picture(guest);
    await guest.wheel(cx, cy, NOTCHES, -120);
    await sleep(900);
    const up = await picture(guest);
    // the top bar's measured values (Memory Usage, CPU Load, Frame Rate) change by themselves: their panel is left out of the check, and nothing else (#1433)
    const measured = topbarMeasuredRect(top.windows);
    const excludedRects = measured ? [measured] : [];
    const cd = changedOutside(top, down, rect, excludedRects), cu = changedOutside(down, up, rect, excludedRects);
    const outDown = cd.n, outUp = cu.n;
    const inDown = measured ? changedIn(top, down, measured).n : null, inUp = measured ? changedIn(down, up, measured).n : null;
    const blank = blankShare(down, rect);
    const ok = outDown === 0 && outUp === 0 && blank < 0.02;
    const where = (b) => (b ? `(${b.x},${b.y}) ${b.w}x${b.h}` : 'none');
    const notes = [...start.notes, `Settings at guest (${st.ax}, ${st.ay}) ${st.w}x${st.h}; changed outside it: down ${outDown} px ${where(cd.box)}, up ${outUp} px ${where(cu.box)}; black share ${(blank * 100).toFixed(2)} %; excluded (top bar's measured values): ${measured ? where(measured) : 'not found'}, changed inside it: down ${inDown ?? 'n/a'} px, up ${inUp ?? 'n/a'} px`];
    log(notes.at(-1));
    return result(ok, (await guest.state())?.state ?? null, notes, { outDown, outUp, outDownBox: cd.box, outUpBox: cu.box, excludedRects, excludedChanged: { down: inDown, up: inUp }, blank: Number(blank.toFixed(4)) });
  },
};
