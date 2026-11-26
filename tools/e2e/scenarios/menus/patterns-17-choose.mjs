// menus/patterns-17-choose: Patterns > #17 (menus/item/patterns-17): the choice shows an effect: a window, a submenu, a changed window or the title. The window it opens is closed after.
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseItem, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/patterns-17-choose',
  covers: ['menus/item/patterns-17'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await chooseItem(ctx, ["Patterns","#17"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
