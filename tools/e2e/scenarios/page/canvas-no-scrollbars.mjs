// page/canvas-no-scrollbars: the page does not scroll: the document is no bigger than the window, so the canvas is never cut by a scroll bar.
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/canvas-no-scrollbars',
  covers: ['page/canvas/no-scrollbars'],
  async run({ page }) {
    const d = await deskInfo(page);
    const s = await page.evaluate(() => ({ sw: document.documentElement.scrollWidth, sh: document.documentElement.scrollHeight, iw: innerWidth, ih: innerHeight }));
    const ok = s.sw <= s.iw + 1 && s.sh <= s.ih + 1;
    return { ok, engineState: d.state, notes: [`document ${s.sw}x${s.sh} CSS px in a window of ${s.iw}x${s.ih}`], results: s };
  },
});
