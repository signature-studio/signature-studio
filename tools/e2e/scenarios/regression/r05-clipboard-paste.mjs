// R05 (#1408) clipboard-paste. Report (L-059, #1389): the browser's clipboard reaches FL's paste. Expectation: navigator.clipboard.writeText('<random>') and then Ctrl+V
// in FL's name edit (the channel's name, F2): the engine's GetClipboardData returns that text (the window-API trace line carries it).
import crypto from 'node:crypto';
import { CHANNEL_NAME_PLACE, grantClipboard, kOf, MAIN, NAME_EDIT, sleep, startDefault, table, tops, traceOn, waitRow, result, engineLines } from './common.mjs';

const SCENARIO = {
  name: 'clipboard-paste',
  report: 'R05',
  expect: 'writeText(random) then Ctrl+V in the channel name edit: the engine GetClipboardData returns the random text',
  expectFail: null,
};

export default {
  ...SCENARIO,
  async run({ guest, page, browserCdp, browserContextId, origin, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    await grantClipboard(browserCdp, browserContextId, origin);
    await traceOn(guest);
    const text = `e2e-${crypto.randomBytes(6).toString('hex')}`;
    await page.evaluate((v) => navigator.clipboard.writeText(v), text);
    const k = await kOf(guest);
    const main = tops(await table(guest)).find((w) => w.cls === MAIN);
    if (!main) return result(false, start.state, [...start.notes, 'no main form']);
    await guest.click(main.ax + CHANNEL_NAME_PLACE.x * k, main.ay + CHANNEL_NAME_PLACE.y * k);
    await sleep(500);
    await guest.key('F2', 'F2', 113);
    const edit = await waitRow(guest, (w) => !w.child && w.cls === NAME_EDIT, 8000);
    const notes = [...start.notes, `name edit open: ${edit.row !== null}`];
    await guest.ctrl('KeyV', 'v', 86);
    await sleep(1500);
    const lines = (await engineLines(guest)).filter((l) => /window-api: GetClipboardData/.test(l));
    const found = lines.some((l) => l.includes(text));
    notes.push(`random text ${text}; GetClipboardData lines: ${lines.length}; the engine received it: ${found}`);
    log(notes.at(-1));
    await guest.escape();
    await sleep(500);
    return result(found, (await guest.state())?.state ?? null, notes, { text, edit: edit.row !== null });
  },
};
