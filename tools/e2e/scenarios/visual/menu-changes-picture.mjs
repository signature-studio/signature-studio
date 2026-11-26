// visual/menu-changes-picture: the File drop-down is drawn: the picture of its rectangle differs from the picture before it opened.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids, openMenu, pixelHashRegion } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'visual/menu-changes-picture',
  covers: [],
  async run({ guest }) {
    const base = await openLids(guest);
    await closeNew(guest, base);
    await sleep(300);
    const barRect = { x: 4, y: 36, w: 208, h: 298 };
    const before = await pixelHashRegion(guest, barRect);
    await openMenu(guest, 'File');
    const { pops } = await newWindows(guest, base);
    const p = pops[0];
    const after = p ? await pixelHashRegion(guest, { x: p.x, y: p.y, w: p.w, h: p.h }) : before;
    await closeNew(guest, base);
    return { ok: !!p && after !== before, engineState: (await guest.state())?.state ?? null, notes: [p ? `drop-down ${p.w}x${p.h} changed the picture: ${after !== before}` : 'no drop-down'] };
  },
});
