// (e) plugin-database-lists-builtins (Issue #1484): the Plugin database (the user's Presets\Plugin database\Installed\) lists the built-in plugins: one entry
// for each plugin folder of lift/plugins/ that has a main DLL (本体; a folder of helpers only has none, Issue #1429). The engine's own listing of that folder
// is the count: its enum-dir line (the number of names the listing returned). The plugins are counted here, from lift/plugins/ (no number is written in this
// file); the names are the folders of the plugins' main DLLs, the same names fe-build writes (plugin_db_names).
import fs from 'node:fs';
import path from 'node:path';
import { closeWelcome, dismissProblems, noThanks, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

// the engine writes its stderr lines with a "[err] " prefix (the page's engine log keeps it)
const LISTING = /^(?:\[err\] )?enum-dir: (\d+) "[^"]*Plugin database\\Installed\\[^"\\]*"$/;

/** the 本体 rows of a plugin.tsv: the rows after the header (a blank line and a # line are no row), as parse_plugin reads them */
export function mainRows(text) {
  const rows = text.split(/\r?\n/).filter((l) => l.trim() !== '' && !l.startsWith('#')).slice(1);
  return rows.map((l) => l.split('\t')).filter((f) => f[2] === '本体');
}

/** the built-in plugins from the plugin.tsv files of lift/plugins/ ({ folder, text }): a folder with a 本体 row is one plugin, named by the folder of its
 *  main DLL; a folder of helpers only (補助) and a file with no row are no plugin (plugin_db_names) */
export function builtinPluginsOf(files) {
  const out = [];
  for (const { folder, text } of files) {
    const [main] = mainRows(text);
    if (!main) continue;
    const parts = main[1].split('\\');
    out.push({ folder, name: parts[parts.length - 2] });
  }
  return out;
}

/** the built-in plugins of the repository: each folder of lift/plugins/ that has a plugin.tsv, in folder order */
export function builtinPlugins(repo) {
  const dir = path.join(repo, 'lift', 'plugins');
  const files = fs.readdirSync(dir).sort().flatMap((folder) => {
    const tsv = path.join(dir, folder, 'plugin.tsv');
    return fs.existsSync(tsv) ? [{ folder, text: fs.readFileSync(tsv, 'utf8') }] : [];
  });
  return builtinPluginsOf(files);
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
    notes.push(`lift/plugins/ has ${plugins.length} plugin(s) with a 本体 row: ${plugins.map((p) => p.name).join(', ')}`);
    notes.push(`the engine's listing of Plugin database\\Installed\\: ${listings.length} line(s), the count ${count}`);
    notes.push(...lines.filter((l) => /Plugin database\\Installed/.test(l)).slice(0, 12).map((l) => `engine: ${l}`));
    const ok = plugins.length > 0 && count >= plugins.length;
    log(`plugin database: ${count} entries listed (${plugins.length} built-in plugins)`);
    return { ok, engineState: end.state, notes };
  },
};
