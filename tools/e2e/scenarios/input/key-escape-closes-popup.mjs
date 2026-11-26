// input/key-escape-closes-popup: a drop-down opened by a click is closed by Escape (no drop-down is new afterwards).
import { defineScenario } from '../../lib/scenario.mjs';
import { newWindows, openLids, openMenu } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'input/key-escape-closes-popup',
  covers: ['menus/menu/file'],
  async run({ guest }) {
    const base = await openLids(guest);
    await openMenu(guest, 'File');
    const before = (await newWindows(guest, base)).pops.length;
    await guest.escape();
    await sleep(600);
    const after = (await newWindows(guest, base)).pops.length;
    return { ok: before >= 1 && after === 0, engineState: (await guest.state())?.state ?? null, notes: [`drop-downs open ${before}, after Escape ${after}`] };
  },
});
