// menus/patterns-18-choose: Patterns > #18 (menus/item/patterns-18): the choice shows an effect: a window, a submenu, a changed window or the title. The window it opens is closed after.
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseItem, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/patterns-18-choose',
  covers: ['menus/item/patterns-18'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await chooseItem(ctx, ["Patterns","#18"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
