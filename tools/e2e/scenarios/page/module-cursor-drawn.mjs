// page/module-cursor-drawn: the guest's cursor is drawn by the page at the device pixel ratio (the page's cursor line names the ratio it is shown at).
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/module-cursor-drawn',
  covers: ['page/module/cursor'],
  async run({ guest, page }) {
    const d = await deskInfo(page);
    await guest.move(await guest.pointAt(Math.round(d.guest.w * 0.5), Math.round(d.guest.h * 0.5)));
    await sleep(800);
    const text = await page.evaluate(() => document.getElementById('cursor-info')?.textContent ?? '');
    const want = `shown at device pixel ratio ${d.dpr}`;
    return { ok: text.includes(want), engineState: d.state, notes: [`cursor line: ${text.slice(0, 180)}`], results: { want, text: text.slice(0, 200) } };
  },
});
