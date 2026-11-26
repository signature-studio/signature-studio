// page/canvas-size-matches-window: the desktop canvas's CSS size is the window's (the room is the whole window), and it sits at the window's corner.
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/canvas-size-matches-window',
  covers: ['page/canvas/size-matches-window'],
  async run({ page }) {
    const d = await deskInfo(page);
    const c = d.canvas;
    const ok = c !== null && Math.abs(c.cssW - d.inner[0]) <= 1 && Math.abs(c.cssH - d.inner[1]) <= 1 && Math.abs(c.left) <= 1 && Math.abs(c.top) <= 1;
    return { ok, engineState: d.state, notes: [`window ${d.inner[0]}x${d.inner[1]} CSS px at DPR ${d.dpr}; canvas ${c ? `${c.cssW.toFixed(2)}x${c.cssH.toFixed(2)} CSS px at ${c.left},${c.top}` : 'none'}`], results: { dpr: d.dpr, inner: d.inner, canvasCss: c ? [c.cssW, c.cssH] : null } };
  },
});
