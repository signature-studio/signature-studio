// page/crisp-random: at each of the two random device pixel ratios (1.01 to 3.00) the canvas is crisp, as at the fixed ratios (the runner draws the two ratios).
import { defineScenario } from '../../lib/scenario.mjs';
import { crispNow } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/crisp-random',
  covers: ['page/crisp/random'],
  displays: ['4k-random'],
  async run({ guest, page }) {
    const r = await crispNow(guest, page);
    return { ok: r.ok, engineState: r.d.state, notes: [`random DPR ${r.c.dpr}: canvas ${r.c.canvas.join('x')} device px, CSS size x DPR ${r.c.cssToDevice.join('x')}, room ${r.c.expected.join('x')}, picture ${r.c.picture.join('x')}`, `resampled ${!r.c.noResample}, off the device grid ${!r.c.onDeviceGrid}`], results: r.c };
  },
});
