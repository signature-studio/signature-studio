// page/crisp-screen-pixels: at DPR 1 the screenshot of the page has the engine's pixels: a sample of the picture equals the screenshot's pixel at the same place (the WebView2 faces are left out). Only at DPR 1: Page.captureScreenshot returns the CSS size (one pixel per CSS px) at other ratios, so the comparison has no device pixels there.
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo, pictureMatchesScreen } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/crisp-screen-pixels',
  covers: ['page/crisp/screen-pixels'],
  displays: ['dpr:1'],
  async run({ guest, page, cdp }) {
    const d = await deskInfo(page);
    const m = await pictureMatchesScreen(guest, cdp, page, d);
    const ok = m.samples > 0 && m.mismatches === 0 && m.shotSize[0] === Math.round(d.inner[0] * d.dpr) && m.shotSize[1] === Math.round(d.inner[1] * d.dpr);
    return { ok, engineState: d.state, notes: [`DPR ${d.dpr}: ${m.samples} samples of the picture, ${m.mismatches} differ from the screenshot; ${m.faces} WebView2 face(s) left out; screenshot ${m.shotSize.join('x')} device px`], results: m };
  },
});
