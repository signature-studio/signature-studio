// input/right-click-mixer: a right-button press and release inside the Mixer (its left column) opens the Mixer's drop-down (tools/e2e/inventory.mjs measured the spot). The Channel rack is not shown after File > New.
import { defineScenario } from '../../lib/scenario.mjs';
import { absoluteRect, childrenOfClass } from '../../lib/dump.mjs';
import { closeNew, newWindows, openLids, windowTable } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/right-click-mixer',
  covers: ['input/mouse/right-click'],
  async run({ guest }) {
    await I.ensureMixer(guest);
    const t = await windowTable(guest);
    const w = childrenOfClass(t, ['TFXForm']).find((c) => c.vis);
    if (!w) return { ok: false, notes: ['no shown Mixer in the window table'] };
    const r = absoluteRect(t, w.lid);
    const base = await openLids(guest);
    await guest.rightClick(r.x + 20, r.y + 60, 60);
    await sleep(800);
    const { pops } = await newWindows(guest, base);
    await closeNew(guest, base);
    return { ok: pops.length >= 1, engineState: (await guest.state())?.state ?? null, notes: [`drop-downs: ${pops.length}`] };
  },
});
