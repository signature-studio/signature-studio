// menus/view-19-submenu-escape: View > #19: Escape closes only its submenu; the drop-down of its parent stays open.
import { defineScenario } from '../../lib/scenario.mjs';
import { submenuEscape, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-19-submenu-escape',
  covers: ['menus/item/view-19'],
  displays: ['4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await submenuEscape(ctx, ["View","#19"]), "displays: 4k-random (the crash-hunt saw this effect at DPR 1.7, none at DPR 1)");
  },
});
