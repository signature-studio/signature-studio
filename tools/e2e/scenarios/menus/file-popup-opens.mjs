// menus/file-popup-opens: the File drop-down opens, and it is taller than one item (its picture box is 100 px high at least).
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids, openMenu } from '../../lib/common.mjs';

export default defineScenario({
  id: 'menus/file-popup-opens',
  covers: ['menus/menu/file'],
  async run({ guest }) {
    const base = await openLids(guest);
    await openMenu(guest, 'File');
    const { pops } = await newWindows(guest, base);
    const p = pops[0];
    await closeNew(guest, base);
    return { ok: !!p && p.h >= 100, engineState: (await guest.state())?.state ?? null, notes: [p ? `File drop-down ${p.w}x${p.h}` : 'no drop-down'] };
  },
});
