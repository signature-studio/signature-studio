// page/focus-desktop-takes-keys: a click on the desktop gives the desktop the keyboard (the focused element is the desktop canvas's box, #desktop).
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/focus-desktop-takes-keys',
  covers: ['page/focus/desktop-takes-keys'],
  async run({ guest, page }) {
    const d = await deskInfo(page);
    await guest.click(Math.round(d.guest.w * 0.9), Math.round(d.guest.h * 0.9));
    await sleep(500);
    const active = await page.evaluate(() => document.activeElement ? document.activeElement.id : null);
    return { ok: active === 'desktop', engineState: d.state, notes: [`after a click on the desktop the focused element is #${active}`], results: { active } };
  },
});
