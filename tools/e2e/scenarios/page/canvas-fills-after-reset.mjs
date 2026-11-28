// page/canvas-fills-after-reset: after File > New (an empty project) the canvas still fills the window and the picture is drawn.
import { defineScenario } from '../../lib/scenario.mjs';
import { resetProject } from '../../lib/common.mjs';
import { deskInfo } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/canvas-fills-after-reset',
  covers: ['page/canvas/fills-window'],
  async run({ guest, page }) {
    const reset = await resetProject(guest);
    const d = await deskInfo(page);
    const c = d.canvas;
    const ok = c !== null && Math.abs(c.cssW - d.inner[0]) <= 1 && Math.abs(c.cssH - d.inner[1]) <= 1 && d.state === 'engine-idle';
    return { ok, engineState: d.state, notes: [`after File > New (left open: ${reset.left.length}); window ${d.inner[0]}x${d.inner[1]}, canvas ${c ? `${c.cssW.toFixed(1)}x${c.cssH.toFixed(1)}` : 'none'} CSS px`], results: { left: reset.left.length } };
  },
});
