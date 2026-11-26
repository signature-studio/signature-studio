// page/console-no-errors-idle: for eight seconds of idle and one change of the picture the page's console has no error and no uncaught exception.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuChangesPicture, pictureHash } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/console-no-errors-idle',
  covers: ['page/console/no-errors'],
  async run({ guest, page }) {
    const errors = [];
    const onConsole = (m) => { if (m.type() === 'error') errors.push(m.text().slice(0, 160)); };
    const onErr = (e) => errors.push(String(e?.message ?? e).slice(0, 160));
    page.on('console', onConsole);
    page.on('pageerror', onErr);
    try {
      await sleep(8000);
      await menuChangesPicture(guest, () => pictureHash(guest));
    } finally {
      page.off('console', onConsole);
      page.off('pageerror', onErr);
    }
    const state = (await guest.state())?.state ?? null;
    return { ok: errors.length === 0, engineState: state, notes: [`console errors and uncaught exceptions in the window: ${errors.length}${errors.length ? `: ${errors.slice(0, 3).join(' | ')}` : ''}`], results: { errors: errors.length } };
  },
});
