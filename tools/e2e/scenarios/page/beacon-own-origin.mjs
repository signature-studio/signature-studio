// page/beacon-own-origin: the beacon goes to the page's own origin only (the request's URL is the page's origin and POST /__perf).
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconNow } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/beacon-own-origin',
  covers: ['page/beacon/own-origin'],
  async run({ guest, page, origin }) {
    const b = await beaconNow(page);
    const u = new URL(b.url);
    const ok = u.origin === new URL(origin).origin && u.pathname === '/__perf';
    const state = (await guest.state())?.state ?? null;
    return { ok, engineState: state, notes: [`beacon to ${u.origin}${u.pathname} (the page's origin ${new URL(origin).origin})`], results: { origin: u.origin, path: u.pathname } };
  },
});
