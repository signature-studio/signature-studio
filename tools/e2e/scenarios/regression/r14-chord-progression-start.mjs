// R14 (#1408) chord-progression-start. Report (L-065, #1386, #1394): the Piano roll's chord progression tool (TOOLS > Chord progression tool) and its "Start creating"
// button do not stop the engine, and the Piano roll gets notes. Expectation: after Start creating the engine is engine-idle (3 s later), and the Piano roll's
// picture changes in its rectangle (more than 500 pixels: the notes that were made).
import { changedIn, openMenuLabel, picture, PIANO_ROLL, sleep, startDefault, TOOL_CHORD, waitRow, result } from './common.mjs';

const SCENARIO = {
  name: 'chord-progression-start',
  report: 'R14',
  expect: 'Piano roll > TOOLS > Chord progression tool > Start creating: engine-idle 3 s after; the Piano roll picture changes (over 500 px) in its rectangle',
  expectFail: '#1386',
};
const TOOLS_LABEL = { fx: 0.819, fy: 0.45 };
const CHORD_DOWNS = 5;
const START_CREATING = { fx: 0.5, fy: 0.847 };
const MIN_CHANGED = 500;

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    await guest.key('F7', 'F7', 118);
    const pr = await waitRow(guest, (w) => w.cls === PIANO_ROLL && /^Piano roll/.test(w.text), 15000);
    if (!pr.row) return result(false, (await guest.state())?.state ?? null, [...start.notes, 'no Piano roll window after F7']);
    const rect = { x: pr.row.ax, y: pr.row.ay, w: pr.row.w, h: pr.row.h };
    const pic0 = await picture(guest);
    const pop = await openMenuLabel(guest, TOOLS_LABEL.fx, TOOLS_LABEL.fy);
    if (!pop) return result(false, (await guest.state())?.state ?? null, [...start.notes, 'the TOOLS drop-down did not open']);
    for (let i = 0; i < CHORD_DOWNS; i++) { await guest.key('ArrowDown', 'ArrowDown', 40); await sleep(250); }
    await guest.key('Enter', 'Enter', 13);
    const tool = await waitRow(guest, (w) => w.cls === TOOL_CHORD, 10000);
    if (!tool.row) return result(false, (await guest.state())?.state ?? null, [...start.notes, 'no chord progression tool window after the item']);
    await guest.click(tool.row.ax + START_CREATING.fx * tool.row.w, tool.row.ay + START_CREATING.fy * tool.row.h);
    await sleep(3000);
    const st = (await guest.state())?.state ?? null;
    const pic1 = await picture(guest);
    const changed = changedIn(pic0, pic1, rect).n;
    const ok = st === 'engine-idle' && changed > MIN_CHANGED;
    const notes = [...start.notes, `Start creating: engine ${st}; Piano roll picture changed ${changed} px in its rectangle (need over ${MIN_CHANGED})`];
    log(notes.at(-1));
    return result(ok, st, notes, { changed });
  },
};
