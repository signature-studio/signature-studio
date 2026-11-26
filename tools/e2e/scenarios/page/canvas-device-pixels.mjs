// page/canvas-device-pixels: the canvas has the room's device pixels: the room (CSS px, the stage) times the device pixel ratio, rounded, inside the engine's limits.
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo, expectedGuest } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/canvas-device-pixels',
  covers: ['page/canvas/device-pixels'],
  async run({ page }) {
    const d = await deskInfo(page);
    const exp = expectedGuest({ w: d.stage.w, h: d.stage.h }, d.dpr);
    const c = d.canvas;
    const ok = c !== null && Math.abs(c.w - exp.w) <= 1 && Math.abs(c.h - exp.h) <= 1;
    return { ok, engineState: d.state, notes: [`room ${d.stage.w}x${d.stage.h} CSS px x DPR ${d.dpr} = ${exp.w}x${exp.h} device px; canvas ${c ? `${c.w}x${c.h}` : 'none'} device px`], results: { dpr: d.dpr, room: [d.stage.w, d.stage.h], expected: [exp.w, exp.h], canvas: c ? [c.w, c.h] : null } };
  },
});
