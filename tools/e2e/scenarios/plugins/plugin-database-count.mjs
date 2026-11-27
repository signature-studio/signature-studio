// plugins/plugin-database-count: the Plugin database's Installed listing gives a count (the engine's enum-dir line: the number of names it returned), and the names: the rows drawn under Installed (named: each built-in plugin of lift/plugins/).
// expectFail new (#1508): the engine writes the listing once per page, so the count is in this scenario's log only when it is the first to open the Plugin database in its page (the order of the shard). Not an insertion failure (no Issue yet).
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineScenario } from '../../lib/scenario.mjs';
import { databaseLines, installedCount, installedRowsShown, openPluginDatabase } from '../../lib/plugins.mjs';
import { builtinPlugins } from '../smoke/plugin-database-lists-builtins.mjs';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');

export default defineScenario({
  id: 'plugins/plugin-database-count',
  covers: ['plugins/list/plugin-database-entries'],
  expectFail: 'new',
  async run({ guest }) {
    const from = await guest.ev('window.__app.engineLog().length');
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const lines = await databaseLines(guest, from);
    const count = installedCount(lines);
    const expected = builtinPlugins(repo).map((p) => p.name);
    const rows = await installedRowsShown(guest);
    const named = Object.fromEntries(expected.map((n) => [n, rows]));
    return {
      ok: count !== null,
      engineState: (await guest.state())?.state ?? null,
      notes: [`Installed listing count: ${count ?? 'none in the engine log'} (lift/plugins: ${expected.length}); rows drawn under Installed: ${rows}`],
      results: { count, expected: expected.length, named },
    };
  },
});
