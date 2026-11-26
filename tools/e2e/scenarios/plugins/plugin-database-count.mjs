// plugins/plugin-database-count: the Plugin database's Installed listing gives a count (the engine's enum-dir line: the number of names it returned).
import { defineScenario } from '../../lib/scenario.mjs';
import { databaseLines, openPluginDatabase } from '../../lib/plugins.mjs';

const COUNT = /enum-dir: (\d+) ".*Installed\\\*\.\*"/;

export default defineScenario({
  id: 'plugins/plugin-database-count',
  covers: ['plugins/list/plugin-database-entries'],
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const lines = await databaseLines(guest);
    const counts = lines.map((l) => COUNT.exec(l)).filter(Boolean).map((m) => Number(m[1]));
    const count = counts.length ? Math.max(...counts) : null;
    return { ok: count !== null, engineState: (await guest.state())?.state ?? null, notes: [`Installed listing count: ${count ?? 'none in the engine log'}`], results: { count } };
  },
});
