// menus/down-moves-highlight: with the File drop-down open, Down changes its picture (the highlight moves to item 1: menus/item/file-1).
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids, openMenu, pixelHashRegion } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'menus/down-moves-highlight',
  covers: ['menus/item/file-1'],
  async run({ guest }) {
    const base = await openLids(guest);
    await openMenu(guest, 'File');
    const { pops } = await newWindows(guest, base);
    const p = pops[0];
    if (!p) { await closeNew(guest, base); return { ok: false, notes: ['no drop-down'] }; }
    const rect = { x: p.x, y: p.y, w: p.w, h: p.h };
    const before = await pixelHashRegion(guest, rect);
    await guest.key('ArrowDown', 'ArrowDown', 40);
    await sleep(400);
    const after = await pixelHashRegion(guest, rect);
    await closeNew(guest, base);
    return { ok: before !== after, engineState: (await guest.state())?.state ?? null, notes: [`picture ${before === after ? 'unchanged' : 'changed'} by Down`] };
  },
});
