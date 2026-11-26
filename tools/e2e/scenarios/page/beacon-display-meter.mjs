// page/beacon-display-meter: the beacon carries the screen meter (the display's frame rate, read by the page) and the engine's rate against it (display and hz are present).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/beacon-display-meter',
  covers: ['page/beacon/display-meter'],
  async run({ guest, page }) {
    const s = await page.evaluate(() => { const b = window.__app.perf.snapshot('periodic'); return { display: b.display !== null, hz: b.hz !== null, rafP95: b.rafIntervalMsP95 ?? null }; });
    const state = (await guest.state())?.state ?? null;
    return { ok: s.display === true && s.hz === true, engineState: state, notes: [`display meter ${s.display}, engine rate ${s.hz}, rAF p95 ${s.rafP95} ms`], results: s };
  },
});
