// (e) plugin-database-lists-builtins (Issue #1484): the Plugin database (the user's Presets\Plugin database\Installed\) lists the built-in plugins: one entry
// for each plugin folder of lift/plugins/ (its main DLL's folder names it). The engine's own listing of that folder is the count: its enum-dir line
// (the number of names the listing returned). The plugins are counted here, from lift/plugins/ (no number is written in this file); the names are the
// folders of the plugins' main DLLs, the same names fe-build writes (plugin_db_names).
import fs from 'node:fs';
import path from 'node:path';
import { closeWelcome, dismissProblems, noThanks, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

// the engine writes its stderr lines with a "[err] " prefix (the page's engine log keeps it)
const LISTING = /^(?:\[err\] )?enum-dir: (\d+) "[^"]*Plugin database\\Installed\\[^"\\]*"$/;

/** the built-in plugins: the folders of lift/plugins/ that have a plugin.tsv; each one's name is the folder of its main DLL (本体) */
export function builtinPlugins(repo) {
  const dir = path.join(repo, 'lift', 'plugins');
  const out = [];
  for (const folder of fs.readdirSync(dir).sort()) {
    const tsv = path.join(dir, folder, 'plugin.tsv');
    if (!fs.existsSync(tsv)) continue;
    const rows = fs.readFileSync(tsv, 'utf8').split('\n').filter((l) => l && !l.startsWith('#')).slice(1);
    const main = rows.map((l) => l.split('\t')).find((f) => f[2] === '本体');
    if (!main) throw new Error(`${folder}/plugin.tsv has no 本体 row`);
    const parts = main[1].split('\\');
    out.push({ folder, name: parts[parts.length - 2] });
  }
  return out;
}

export default {
  name: 'plugin-database-lists-builtins',
  expectFail: null,
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
    const plugins = builtinPlugins(repo);
    const lines = await guest.ev('window.__app.engineLog()');
    const listings = lines.map((l) => LISTING.exec(l)).filter(Boolean);
    const count = listings.reduce((m, x) => Math.max(m, Number(x[1])), 0);
    notes.push(`lift/plugins/ has ${plugins.length} plugin(s): ${plugins.map((p) => p.name).join(', ')}`);
    notes.push(`the engine's listing of Plugin database\\Installed\\: ${listings.length} line(s), the count ${count}`);
    notes.push(...lines.filter((l) => /Plugin database\\Installed/.test(l)).slice(0, 12).map((l) => `engine: ${l}`));
    const ok = plugins.length > 0 && count >= plugins.length;
    log(`plugin database: ${count} entries listed (${plugins.length} built-in plugins)`);
    return { ok, engineState: end.state, notes };
  },
};
