// page/focus-blur-then-focus: the window loses the focus (blur) and gets it back (focus): after it is back the engine draws again (a change of the picture).
import { defineScenario } from '../../lib/scenario.mjs';
import { menuChangesPicture, pictureHash, setFocused } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/focus-blur-then-focus',
  covers: ['page/focus/blur-then-focus'],
  async run({ guest, page }) {
    try {
      const lost = await setFocused(page, false);
      await sleep(1200);
      await setFocused(page, true);
      await sleep(600);
      const f = await menuChangesPicture(guest, () => pictureHash(guest));
      const drawn = f.pops > 0 && f.shown !== f.after;
      const state = (await guest.state())?.state ?? null;
      return { ok: lost === false && drawn && state === 'engine-idle', engineState: state, notes: [`blur: the page has focus ${lost}; focus back; the File menu drew ${drawn}; engine ${state}`], results: { lost, drawn } };
    } finally {
      await setFocused(page, true);
      await sleep(400);
    }
  },
});
