// menus/key-f7: F7: the effect in the window table (a window, a submenu, a changed window or the title). No menu item of the list is named as its duplicate.
import { defineScenario } from '../../lib/scenario.mjs';
import { shortcut, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/key-f7',
  covers: [],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await shortcut(ctx, 'F7', async (g) => { await g.key('F7', 'F7', 118); }, null), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
