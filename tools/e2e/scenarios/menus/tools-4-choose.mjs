// menus/tools-4-choose: Tools > #4 (menus/item/tools-4): the choice shows an effect: a window, a submenu, a changed window or the title. The window it opens is closed after.
// crash-hunt: the engine stops on this item (new)
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseItem, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/tools-4-choose',
  covers: ['menus/item/tools-4'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  async run(ctx) {
    return withNote(await chooseItem(ctx, ["Tools","#4"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
