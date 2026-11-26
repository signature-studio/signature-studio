// page/html-no-data-image: the page's text has no image written out as text (no data: picture; a cursor's picture is never a text on the page).
import { defineScenario } from '../../lib/scenario.mjs';
import { VISIBLE_CHECK } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/html-no-data-image',
  covers: ['page/html/no-data-image'],
  async run({ guest, page }) {
    const v = await page.evaluate(VISIBLE_CHECK);
    const state = (await guest.state())?.state ?? null;
    return { ok: v.dataImage === false, engineState: state, notes: [`data:image in the page's text: ${v.dataImage ? 'yes' : 'no'}`], results: { dataImage: v.dataImage } };
  },
});
