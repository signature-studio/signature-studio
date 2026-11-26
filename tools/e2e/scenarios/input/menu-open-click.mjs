// input/menu-open-click: a left click on the File label of the menu bar opens its drop-down (a new TQuickPopupMenuWindow below the bar).
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids, openMenu } from '../../lib/common.mjs';

export default defineScenario({
  id: 'input/menu-open-click',
  covers: ['menus/menu/file'],
  async run({ guest }) {
    const base = await openLids(guest);
    await openMenu(guest, 'File');
    const { pops } = await newWindows(guest, base);
    const ok = pops.length >= 1 && pops[0].h > 100;
    const left = await closeNew(guest, base);
    return { ok: ok && left.length === 0, engineState: (await guest.state())?.state ?? null, notes: [`drop-downs after the click: ${pops.map((p) => `${p.w}x${p.h} at ${p.x},${p.y}`).join('; ') || 'none'}`] };
  },
});
