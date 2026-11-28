// menus/key-f8: F8 and View > #6 (menus/item/view-6): the two paths give the same kind of effect (a window, a submenu, a changed window or the title).
import { defineScenario } from '../../lib/scenario.mjs';
import { keyAndMenu, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/key-f8',
  covers: ['menus/item/view-6'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await keyAndMenu(ctx, 'F8', async (g) => { await g.key('F8', 'F8', 119); }, ["View","#6"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
