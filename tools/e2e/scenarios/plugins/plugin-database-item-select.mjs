// plugins/plugin-database-item-select: a click on the Installed row of the Plugin database selects it: the Browser's information box shows its title (the picture of the Browser changes).
import { defineScenario } from '../../lib/scenario.mjs';
import { openPluginDatabase } from '../../lib/plugins.mjs';
import { pixelHashRegion } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

const BROWSER = { x: 0, y: 82, w: 372, h: 300 };
const INSTALLED = { x: 60, y: 182 };

export default defineScenario({
  id: 'plugins/plugin-database-item-select',
  covers: ['plugins/select/plugin-database-item'],
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const before = await pixelHashRegion(guest, BROWSER);
    await guest.click(INSTALLED.x, INSTALLED.y);
    await sleep(1500);
    const after = await pixelHashRegion(guest, BROWSER);
    return { ok: before !== after, engineState: (await guest.state())?.state ?? null, notes: [`the Browser's picture ${before !== after ? 'changed' : 'did not change'} after the Installed row was clicked`] };
  },
});
