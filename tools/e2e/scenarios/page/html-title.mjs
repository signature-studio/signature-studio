// page/html-title: the page's title names FL Studio and the project (the title the page shows in its tab).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/html-title',
  covers: ['page/html/title'],
  async run({ guest, page }) {
    const t = await page.title();
    const state = (await guest.state())?.state ?? null;
    return { ok: t.includes('FL Studio') && t.length > 'FL Studio'.length, engineState: state, notes: [`title "${t}"`], results: { title: t } };
  },
});
