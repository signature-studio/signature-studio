// page/crisp-dpr-2-5: at DPR 2.5 the canvas is crisp: its CSS size times 2.5 is its pixels, its pixels are the room's device pixels, and the picture has its size.
import { defineScenario } from '../../lib/scenario.mjs';
import { crispNow } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/crisp-dpr-2-5',
  covers: ['page/crisp/dpr-2-5'],
  displays: ['dpr:2.5'],
  async run({ guest, page }) {
    const r = await crispNow(guest, page);
    return { ok: r.ok, engineState: r.d.state, notes: [`DPR ${r.c.dpr}: canvas ${r.c.canvas.join('x')} device px, CSS size x DPR ${r.c.cssToDevice.join('x')}, room ${r.c.expected.join('x')}, picture ${r.c.picture.join('x')}`, `resampled ${!r.c.noResample}, off the device grid ${!r.c.onDeviceGrid}`], results: r.c };
  },
});
