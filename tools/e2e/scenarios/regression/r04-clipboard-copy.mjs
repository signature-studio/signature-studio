// R04 (#1408) clipboard-copy. Report (L-059, #1389): the Copy of a knob's menu puts the text into the browser's clipboard. Expectation: the engine's SetClipboardData
// (the window-API trace line, fmt 13, carries the text) is non-empty, and the page's navigator.clipboard.readText() returns the same text after the Copy.
import path from 'node:path';
import { clickAt, grantClipboard, kOf, MAIN, newTops, POPUP, sleep, startDefault, table, tops, traceOn, under, result, engineLines } from './common.mjs';

const SCENARIO = {
  name: 'clipboard-copy',
  report: 'R04',
  expect: 'Copy in the Volume knob menu: the engine hands a text to SetClipboardData, and navigator.clipboard.readText() returns the same text',
  expectFail: null,
};
// the first channel's Volume knob, from the main form's top-left corner (as the knob scenario, at DPR 1)
const VOLUME_KNOB = { dx: 406, dy: 145 };
// the knob menu's "Copy value" row, from the menu's top-left corner (guest px at DPR 1: the row's middle, read from the picture of the menu; the menu's items
// are not windows, so their captions are not in the window table)
const COPY_AT = { dx: 60, dy: 253 };

export default {
  ...SCENARIO,
  async run({ guest, page, browserCdp, browserContextId, origin, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    await grantClipboard(browserCdp, browserContextId, origin);
    await traceOn(guest);
    const k = await kOf(guest);
    const main = tops(await table(guest)).find((w) => w.cls === MAIN);
    if (!main) return result(false, start.state, [...start.notes, 'no main form']);
    const before = (await table(guest)).map((w) => w.lid);
    await guest.rightClick(main.ax + VOLUME_KNOB.dx * k, main.ay + VOLUME_KNOB.dy * k);
    await sleep(1200);
    const t1 = await table(guest);
    const pop = newTops(t1, before).find((w) => w.cls === POPUP);
    if (!pop) return result(false, (await guest.state())?.state ?? null, [...start.notes, 'the knob menu did not open']);
    const notes = [...start.notes, `knob menu at guest (${pop.ax}, ${pop.ay}) ${pop.w}x${pop.h}`];
    if (guest.shots) await guest.shot(path.join(guest.shots, 'knob-menu.png'));
    const at = { x: pop.ax + COPY_AT.dx * k, y: pop.ay + COPY_AT.dy * k };
    notes.push(`clicking "Copy value" at guest (${Math.round(at.x)}, ${Math.round(at.y)})`);
    await clickAt(guest, at.x, at.y);
    await sleep(1500);
    const lines = (await engineLines(guest)).filter((l) => /window-api: SetClipboardData fmt=13 .*text="/.test(l));
    const m = lines.length ? /text="(.*)"$/.exec(lines.at(-1)) : null;
    const engineText = m ? m[1] : null;
    const read = await page.evaluate(() => navigator.clipboard.readText().then((s) => s, (e) => `ERR ${e}`));
    const match = engineText !== null && engineText.length > 0 && read === engineText;
    notes.push(`engine SetClipboardData: ${engineText === null ? 'none' : JSON.stringify(engineText.slice(0, 80))}; navigator.clipboard.readText(): ${JSON.stringify(String(read).slice(0, 80))}; match ${match}`);
    log(notes.at(-1));
    return result(match, (await guest.state())?.state ?? null, notes, { engineText: engineText?.slice(0, 200) ?? null, readText: String(read).slice(0, 200) });
  },
};
