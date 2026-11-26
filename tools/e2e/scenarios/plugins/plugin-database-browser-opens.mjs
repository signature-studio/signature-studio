// plugins/plugin-database-browser-opens: the picker's Plugin database item opens the Browser's Plugin database; the engine lists its Installed folder (a listing line is in the engine's log).
import { defineScenario } from '../../lib/scenario.mjs';
import { databaseLines, openPluginDatabase } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/plugin-database-browser-opens',
  covers: ['plugins/window/plugin-database', 'plugins/tool/plugin-database'],
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const lines = await databaseLines(guest);
    const listing = lines.some((l) => /enum-dir: \d+ ".*Installed/.test(l));
    return { ok: listing, engineState: (await guest.state())?.state ?? null, notes: [`Plugin database lines in the engine log: ${lines.length}; a listing of Installed: ${listing}`] };
  },
});
