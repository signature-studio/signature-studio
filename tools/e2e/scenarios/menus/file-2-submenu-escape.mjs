// menus/file-2-submenu-escape: File > #2: Escape closes only its submenu; the drop-down of its parent stays open.
import { defineScenario } from '../../lib/scenario.mjs';
import { submenuEscape, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/file-2-submenu-escape',
  covers: ['menus/item/file-2'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await submenuEscape(ctx, ["File","#2"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
