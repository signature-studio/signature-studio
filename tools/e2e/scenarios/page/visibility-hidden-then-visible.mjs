// page/visibility-hidden-then-visible: the page is hidden (visibilitychange, the page reads hidden) and then visible again: after it is visible the engine draws again (a change of the picture).
import { defineScenario } from '../../lib/scenario.mjs';
import { menuChangesPicture, pictureHash, setHidden } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/visibility-hidden-then-visible',
  covers: ['page/visibility/hidden-then-visible'],
  async run({ guest, page }) {
    try {
      const h = await setHidden(page, true);
      await sleep(2500);
      const hiddenState = (await guest.state())?.state ?? null;
      await setHidden(page, false);
      await sleep(800);
      const f = await menuChangesPicture(guest, () => pictureHash(guest));
      const drawn = f.pops > 0 && f.shown !== f.after;
      const state = (await guest.state())?.state ?? null;
      return { ok: h === 'hidden' && drawn && state === 'engine-idle' && hiddenState !== 'error', engineState: state, notes: [`hidden: the page reads ${h}; engine ${hiddenState} while hidden; visible again; the File menu drew ${drawn}`], results: { hidden: h, drawn, engine: state } };
    } finally {
      await setHidden(page, false);
      await sleep(500);
    }
  },
});
