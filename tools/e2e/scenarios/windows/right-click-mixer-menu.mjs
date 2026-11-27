// windows/right-click-mixer-menu: the right-click drop-down of the Mixer, as the inventory measured it (windows.tsv: windows/popup/right-click-mixer).
import { defineScenario } from '../../lib/scenario.mjs';
import { absoluteRect, childrenOfClass } from '../../lib/dump.mjs';
import { closeNew, newWindows, openLids, windowTable } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'windows/right-click-mixer-menu',
  covers: ['windows/popup/right-click-mixer'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const w = childrenOfClass(t, ['TFXForm']).find((c) => c.vis);
    if (!w) return { ok: false, notes: ['no shown Mixer'] };
    const r = absoluteRect(t, w.lid);
    const base = await openLids(guest);
    await guest.rightClick(r.x + 20, r.y + 60, 60);
    await sleep(800);
    const { pops } = await newWindows(guest, base);
    await closeNew(guest, base);
    return { ok: pops.length >= 1, engineState: (await guest.state())?.state ?? null, notes: [`drop-downs ${pops.length}`] };
  },
});
