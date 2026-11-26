// menus/view-11-1-choose: View > #11 > #1 (menus/item/view-11-1): the choice shows an effect: a window, a submenu, a changed window or the title. The window it opens is closed after.
// crash-hunt: the engine stops on this item (#1416)
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseItem, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-11-1-choose',
  covers: ['menus/item/view-11-1'],
  displays: ['4k-dpr1'],
  expectFail: '#1416',
  async run(ctx) {
    return withNote(await chooseItem(ctx, ["View","#11","#1"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
