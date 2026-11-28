// resilience/tab-hidden-resume: the page's tab is hidden (a second tab of the same browser is brought to the front), kept hidden for 30 s, then shown again.
// Expected: the page is hidden while the other tab is in front and visible again after; after it, drawing and the keys go on (F6 changes the picture); no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { sleep } from '../../lib/flow.mjs';
import { deskHash, hideTab, press, showTab, stateName, waitUntil } from '../../lib/resilience.mjs';

const HIDDEN_MS = 30000;

export default defineScenario({
  id: 'resilience/tab-hidden-resume',
  covers: ['resilience/tab/hidden-resume'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const { other, hidden } = await hideTab(page);
    const t0 = Date.now();
    let stops = 0;
    while (Date.now() - t0 < HIDDEN_MS) {
      const s = await stateName(guest);
      if (s === 'error' || s === 'exited') { stops++; break; }
      await sleep(2000);
    }
    const hiddenMs = Date.now() - t0;
    const visible = await showTab(page, other);
    await guest.focusDesktop();
    const before = await deskHash(guest);
    await press(guest, 'F6');
    const changed = await waitUntil(async () => (await deskHash(guest)) !== before, 8000, 300);
    const state = await stateName(guest);
    return {
      ok: hidden && visible && changed && stops === 0 && state === 'engine-idle', engineState: state,
      notes: [`hidden ${hidden} for ${hiddenMs} ms (stops ${stops}), visible again ${visible}, a key after it changes the picture ${changed}`],
      results: { hiddenMs, hidden, visible, drawsAfter: changed, stops },
    };
  },
});
