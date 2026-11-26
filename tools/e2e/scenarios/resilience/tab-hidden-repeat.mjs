// resilience/tab-hidden-repeat: five times, the page's tab is hidden for 8 s and shown again, and a key after each show-up changes the picture. Expected: each time hidden, visible
// again, and drawing and keys go on; no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { sleep } from '../../lib/flow.mjs';
import { deskHash, hideTab, press, showTab, stateName, waitUntil } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/tab-hidden-repeat',
  covers: ['resilience/tab/hidden-repeat'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const rows = [];
    for (let i = 1; i <= 5; i++) {
      const { other, hidden } = await hideTab(page);
      await sleep(8000);
      const visible = await showTab(page, other);
      await guest.focusDesktop();
      const before = await deskHash(guest);
      await press(guest, 'F6');
      const changed = await waitUntil(async () => (await deskHash(guest)) !== before, 8000, 300);
      rows.push({ i, hidden, visible, changed, state: await stateName(guest) });
    }
    const good = rows.filter((r) => r.hidden && r.visible && r.changed && r.state === 'engine-idle').length;
    const state = await stateName(guest);
    return {
      ok: good === 5 && state === 'engine-idle', engineState: state,
      notes: [`5 hide and show: hidden, visible and drawing after each ${good} times`],
      results: { cycles: 5, good, rows },
    };
  },
});
