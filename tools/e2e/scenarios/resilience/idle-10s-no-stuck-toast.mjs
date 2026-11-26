// resilience/idle-10s-no-stuck-toast: nothing is sent for 10 s; then no live region (status, alert, aria-live) of the page has stayed visible with the same text for more than 10 s.
// Expected: the stuck count is 0, no stop; the engine idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { sleep } from '../../lib/flow.mjs';
import { toastState } from '../../lib/detect.mjs';
import { stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/idle-10s-no-stuck-toast',
  covers: ['resilience/soak/idle-10s-no-stuck-toast'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const t0 = Date.now();
    let stops = 0;
    while (Date.now() - t0 < 10000) {
      const s = await stateName(guest);
      if (s === 'error' || s === 'exited') { stops++; break; }
      await sleep(1000);
    }
    const t = await toastState(page);
    const state = await stateName(guest);
    return {
      ok: t.stuck === 0 && stops === 0 && state === 'engine-idle', engineState: state,
      notes: [`10 s with nothing sent: stuck live regions ${t.stuck} (${JSON.stringify(t.ids)}), stops ${stops}, live regions visible now ${t.visible}`],
      results: { stuck: t.stuck, stuckIds: t.ids, visibleNow: t.visible, stops },
    };
  },
});
