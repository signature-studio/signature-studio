// menus/edit-popup-opens: the Edit drop-down opens (the second label of the bar).
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids, openMenu } from '../../lib/common.mjs';

export default defineScenario({
  id: 'menus/edit-popup-opens',
  covers: ['menus/menu/edit'],
  async run({ guest }) {
    const base = await openLids(guest);
    await openMenu(guest, 'Edit');
    const { pops } = await newWindows(guest, base);
    const p = pops[0];
    await closeNew(guest, base);
    return { ok: !!p, engineState: (await guest.state())?.state ?? null, notes: [p ? `Edit drop-down ${p.w}x${p.h}` : 'no drop-down'] };
  },
});
