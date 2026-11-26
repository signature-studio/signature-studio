// topbar/menu-bar-visible: the menu bar (TNewMenu) is a shown child of the top bar's toolbar panel.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'topbar/menu-bar-visible',
  covers: ['topbar/part/tnewmenu'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const bar = t.children.find((c) => c.cls === 'TNewMenu');
    return { ok: !!bar && bar.vis && bar.w > 100, engineState: (await guest.state())?.state ?? null, notes: [bar ? `menu bar ${bar.w}x${bar.h} vis ${bar.vis}` : 'no TNewMenu'] };
  },
});
