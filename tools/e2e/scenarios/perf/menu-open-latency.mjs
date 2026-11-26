// perf/menu-open-latency: from the click on the File label to the drop-down in the window table: under 10 s (the runner has no GPU and reads the table by polling; the value is recorded).
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids, openMenu } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'perf/menu-open-latency',
  covers: [],
  async run({ guest }) {
    const base = await openLids(guest);
    const t0 = Date.now();
    await openMenu(guest, 'File');
    let ms = null;
    for (let i = 0; i < 60 && ms === null; i++) {
      if ((await newWindows(guest, base)).pops.length) ms = Date.now() - t0;
      else await sleep(50);
    }
    await closeNew(guest, base);
    return { ok: ms !== null && ms < 10000, engineState: (await guest.state())?.state ?? null, notes: [`drop-down after ${ms ?? 'never'} ms (runner value)`] };
  },
});
