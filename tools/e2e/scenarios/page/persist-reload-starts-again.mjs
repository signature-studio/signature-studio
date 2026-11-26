// page/persist-reload-starts-again: the page is reloaded; the start runs again to the idle engine and the default project (lib/common.mjs persist).
import { defineScenario } from '../../lib/scenario.mjs';
import { persist } from '../../lib/common.mjs';

export default defineScenario({
  id: 'page/persist-reload-starts-again',
  covers: ['page/module/persist'],
  async run({ guest, page }) {
    const r = await persist(page, guest);
    return { ok: r.opened === true && r.state === 'engine-idle', engineState: r.state, notes: [`title before "${r.titleBefore}", after "${r.titleAfter}"`] };
  },
});
