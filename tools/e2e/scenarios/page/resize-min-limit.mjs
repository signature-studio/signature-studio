// page/resize-min-limit: a window smaller than the engine's smallest screen (640 x 480 device px): the guest's screen is 640 x 480, and that is shown fitted in the window.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, deskInfo } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/resize-min-limit',
  covers: ['page/resize/min-limit'],
  async run({ page }) {
    const base = await baseOf(page);
    const w = Math.floor(500 / base.dpr), h = Math.floor(400 / base.dpr);
    try {
      await page.setViewportSize({ width: w, height: h });
      let d = null;
      for (let i = 0; i < 60; i++) { await sleep(250); d = await deskInfo(page); if (d.guest && d.guest.w === 640 && d.guest.h === 480 && d.canvas && d.canvas.w === 640) break; }
      const fits = d.canvas !== null && d.canvas.cssW <= d.stage.w + 1 && d.canvas.cssH <= d.stage.h + 1;
      const ok = d.guest !== null && d.guest.w === 640 && d.guest.h === 480 && fits;
      return { ok, engineState: d.state, notes: [`window ${w}x${h} CSS px at DPR ${base.dpr}: guest ${d.guest.w}x${d.guest.h}, canvas ${d.canvas.w}x${d.canvas.h} device px shown at ${d.canvas.cssW.toFixed(1)}x${d.canvas.cssH.toFixed(1)} CSS px in the room ${d.stage.w}x${d.stage.h}; fits ${fits}`], results: { guest: [d.guest.w, d.guest.h], canvas: [d.canvas.w, d.canvas.h], fits } };
    } finally {
      await page.setViewportSize({ width: base.w, height: base.h });
      await sleep(1500);
    }
  },
});
