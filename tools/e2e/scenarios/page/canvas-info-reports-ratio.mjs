// page/canvas-info-reports-ratio: the page's line about the canvas names the device pixel ratio and the guest's screen in device pixels (the page's own report, read as text).
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/canvas-info-reports-ratio',
  covers: ['page/canvas/info-reports-ratio'],
  async run({ page }) {
    const d = await deskInfo(page);
    const want = `device pixel ratio ${d.dpr}`;
    const ok = d.info.includes(want) && d.info.includes(`guest screen ${d.guest.w}x${d.guest.h} device px`);
    return { ok, engineState: d.state, notes: [`want "${want}"; the page says: ${d.info.slice(0, 160)}`], results: { dpr: d.dpr, info: d.info } };
  },
});
