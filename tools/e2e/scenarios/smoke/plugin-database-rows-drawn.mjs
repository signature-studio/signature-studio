// (f) plugin-database-rows-drawn (Issue #1535): the Plugin database's Installed folder lists its category folders and draws its rows. The picker's Plugin database
// item opens the Browser's Plugin database; Installed is then opened (its row, then Right). The engine's enum-dir lines name the listed folders: the Effects folder
// under Installed must list each built-in effect's .nfo and .fst (lift/plugins: a 本体 row under Effects). The rows themselves are read by eye on the picture
// (the report says so); the picture's hash and the text table ("Directory is empty!" is always in the memory) are not judges (#1535).
import fs from 'node:fs';
import path from 'node:path';
import { closeWelcome, dismissProblems, noThanks, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';
import { mainRows } from './plugin-database-lists-builtins.mjs';
import { categoryCount, databaseLines, hintSaysEmpty, openInstalled, openPluginDatabase } from '../../lib/plugins.mjs';

/** the built-in effects of the repository: the folders of lift/plugins/ whose 本体 row lies under an Effects folder */
export function builtinEffectCount(repo) {
  const dir = path.join(repo, 'lift', 'plugins');
  return fs.readdirSync(dir).sort().filter((folder) => {
    const tsv = path.join(dir, folder, 'plugin.tsv');
    if (!fs.existsSync(tsv)) return false;
    const [main] = mainRows(fs.readFileSync(tsv, 'utf8'));
    return main !== undefined && main[1].split('\\').includes('Effects');
  }).length;
}

export default {
  name: 'plugin-database-rows-drawn',
  expectFail: null,
  displays: ['4k-dpr1', '4k-dpr2'],
  async run({ guest, repo, log }) {
    const notes = [];
    const first = await waitStable(guest, 180000);
    notes.push(`first state ${first.state} after ${first.waitedMs} ms`);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes };
    if (await guest.findWindow((w) => w.cls === WELCOME)) {
      await noThanks(guest);
      await closeWelcome(guest);
    }
    const end = await waitProjectOpen(guest, first.title, 120000);
    if (!end.opened) return { ok: false, engineState: end.state, notes: [...notes, 'the default project did not open'] };
    await dismissProblems(guest);
    const from = await guest.ev('window.__app.engineLog().length');
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [...notes, `the picker did not open (${db.error})`] };
    await openInstalled(guest);
    const lines = await databaseLines(guest, from);
    const effects = categoryCount(lines, 'Effects');
    const expected = builtinEffectCount(repo);
    const state = (await guest.state())?.state ?? null;
    const memoryText = await hintSaysEmpty(guest);
    notes.push(`the Installed folder's Effects listing: ${effects === null ? 'none' : `${effects} name(s)`} (each built-in effect is a .nfo and a .fst: at least ${2 * expected})`);
    notes.push(`"Directory is empty!" is in the guest's memory: ${memoryText} (the text table holds it: not a judge; the picture is read by eye)`);
    notes.push(`the Plugin database lines under Installed: ${lines.length}`);
    log(`plugin database rows: Effects listing ${effects} of ${expected}`);
    return { ok: effects !== null && effects >= 2 * expected && state === 'engine-idle', engineState: state, notes };
  },
};
