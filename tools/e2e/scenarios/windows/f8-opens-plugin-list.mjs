// windows/f8-opens-plugin-list: F8 opens the plugin list (a top-level window of class TPlugListForm; the inventory's windows.tsv has it); Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'windows/f8-opens-plugin-list',
  covers: ['windows/window/tpluglistform'],
  async run({ guest }) {
    const base = await openLids(guest);
    await guest.focusDesktop();
    await guest.key('F8', 'F8', 119);
    await sleep(1500);
    const { others } = await newWindows(guest, base);
    const w = others.find((x) => x.cls === 'TPlugListForm');
    const left = await closeNew(guest, base);
    await sleep(300);
    return { ok: !!w && left.length === 0, engineState: (await guest.state())?.state ?? null, notes: [w ? `opened ${w.cls} ${w.w}x${w.h}` : 'no TPlugListForm opened by F8'] };
  },
});
