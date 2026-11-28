// R15 (#1408) riff-machine-open. Report (L-066, #1387, #1400): the Riff machine (TOOLS > Riff machine, from the Piano roll's TOOLS menu) opens a window when it
// is started. Expectation: a new top-level window whose caption contains "riff" (the Piano roll's riff machine: "Piano roll - riff machine") appears within 15 s, and the engine is engine-idle 3 s after it.
import { openMenuLabel, PIANO_ROLL, sleep, startDefault, table, waitRow, result } from './common.mjs';

const SCENARIO = {
  name: 'riff-machine-open',
  report: 'R15',
  expect: 'Piano roll > TOOLS > Riff machine: a new top-level window whose caption contains "riff" appears; the engine is engine-idle 3 s after',
  // the window opens and the engine stays idle: the stop on an indirect call (#1429) is closed and no longer seen, so no mark (#1387 is closed on main too)
  expectFail: null,
};
const TOOLS_LABEL = { fx: 0.819, fy: 0.45 };
const RIFF_DOWNS = 4;

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    await guest.key('F7', 'F7', 118);
    const pr = await waitRow(guest, (w) => w.cls === PIANO_ROLL && /^Piano roll/.test(w.text), 15000);
    if (!pr.row) return result(false, (await guest.state())?.state ?? null, [...start.notes, 'no Piano roll window after F7']);
    const before = (await table(guest)).map((w) => w.lid);
    const pop = await openMenuLabel(guest, TOOLS_LABEL.fx, TOOLS_LABEL.fy);
    if (!pop) return result(false, (await guest.state())?.state ?? null, [...start.notes, 'the TOOLS drop-down did not open']);
    for (let i = 0; i < RIFF_DOWNS; i++) { await guest.key('ArrowDown', 'ArrowDown', 40); await sleep(250); }
    await guest.key('Enter', 'Enter', 13);
    const riff = await waitRow(guest, (w) => !w.child && w.vis && /riff/i.test(w.text) && !before.includes(w.lid), 15000);
    await sleep(3000);
    const st = (await guest.state())?.state ?? null;
    const opened = riff.row !== null;
    const notes = [...start.notes, `Riff machine window: ${opened ? `"${riff.row.text}" at guest (${riff.row.ax}, ${riff.row.ay}) ${riff.row.w}x${riff.row.h}` : 'none'}; engine ${st}`];
    log(notes.at(-1));
    return result(opened && st === 'engine-idle', st, notes, { opened });
  },
};
