// page/visibility-presents-resume: after the page is hidden and visible again the presenter counts new presents (the picture is shown again, not only kept).
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo, menuChangesPicture, pictureHash, setHidden } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/visibility-presents-resume',
  covers: ['page/visibility/presents-resume'],
  async run({ guest, page }) {
    try {
      const before = await deskInfo(page);
      await setHidden(page, true);
      await sleep(2000);
      await setHidden(page, false);
      await sleep(800);
      const f = await menuChangesPicture(guest, () => pictureHash(guest));
      await sleep(800);
      const after = await deskInfo(page);
      const more = after.presents !== null && before.presents !== null && after.presents > before.presents;
      const drawn = f.pops > 0 && f.shown !== f.after;
      return { ok: more && drawn, engineState: after.state, notes: [`presents ${before.presents} -> ${after.presents} (more ${more}); the File menu drew ${drawn}`], results: { before: before.presents, after: after.presents, drawn } };
    } finally {
      await setHidden(page, false);
      await sleep(500);
    }
  },
});
