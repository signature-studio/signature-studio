// topbar/toolbar-panels-present: the top bar holds its toolbar panels (TQuickEditToolbarPanel, at least 10 under the main form).
import { defineScenario } from '../../lib/scenario.mjs';
import { MAIN_FORM, windowTable } from '../../lib/common.mjs';
import { subtree } from '../../lib/dump.mjs';

export default defineScenario({
  id: 'topbar/toolbar-panels-present',
  covers: ['topbar/part/tquickedittoolbarpanel'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const main = t.top.find((w) => w.cls === MAIN_FORM);
    const n = main ? subtree(t, main.lid).filter((c) => c.cls === 'TQuickEditToolbarPanel').length : 0;
    return { ok: n >= 10, engineState: (await guest.state())?.state ?? null, notes: [`toolbar panels under the main form: ${n}`] };
  },
});
