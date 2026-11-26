// menus/tools-12-choose: Tools > #12 (menus/item/tools-12): the choice shows an effect: a window, a submenu, a changed window or the title. The window it opens is closed after.
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseItem, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/tools-12-choose',
  covers: ['menus/item/tools-12'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await chooseItem(ctx, ["Tools","#12"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
