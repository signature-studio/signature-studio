// page/canvas-pointer-maps-guest: a pointer at a guest pixel is that pixel of the engine's screen (the page's pointer line reads the same guest pixel, within 1).
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/canvas-pointer-maps-guest',
  covers: ['page/canvas/pointer-maps-guest'],
  async run({ guest, page }) {
    const d = await deskInfo(page);
    const gx = Math.round(d.guest.w * 0.25), gy = Math.round(d.guest.h * 0.5);
    await guest.move(await guest.pointAt(gx, gy));
    await sleep(500);
    const text = await page.evaluate(() => document.getElementById('pointer-info')?.textContent ?? '');
    const m = /screen \((-?\d+), (-?\d+)\)/.exec(text);
    const ok = m !== null && Math.abs(Number(m[1]) - gx) <= 1 && Math.abs(Number(m[2]) - gy) <= 1;
    return { ok, engineState: d.state, notes: [`pointer at guest ${gx},${gy}; the page reads: ${text.slice(0, 80)}`], results: { want: [gx, gy], read: m ? [Number(m[1]), Number(m[2])] : null } };
  },
});
