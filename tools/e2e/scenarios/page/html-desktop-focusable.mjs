// page/html-desktop-focusable: the desktop element (#desktop) can take the keyboard (tabindex 0) and names the application for assistive technology.
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/html-desktop-focusable',
  covers: ['page/html/desktop-focusable'],
  async run({ guest, page }) {
    const a = await page.evaluate(() => { const el = document.getElementById('desktop'); return el ? { tab: el.getAttribute('tabindex'), role: el.getAttribute('role'), label: el.getAttribute('aria-label') } : null; });
    const state = (await guest.state())?.state ?? null;
    const ok = a !== null && a.tab === '0' && a.role === 'application' && typeof a.label === 'string' && a.label.length > 0;
    return { ok, engineState: state, notes: [`#desktop tabindex ${a?.tab}, role ${a?.role}, label "${a?.label ?? ''}"`], results: a };
  },
});
