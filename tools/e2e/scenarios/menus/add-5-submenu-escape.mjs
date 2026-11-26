// menus/add-5-submenu-escape: Add > #5: Escape closes only its submenu; the drop-down of its parent stays open.
import { defineScenario } from '../../lib/scenario.mjs';
import { submenuEscape, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/add-5-submenu-escape',
  covers: ['menus/item/add-5'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await submenuEscape(ctx, ["Add","#5"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
