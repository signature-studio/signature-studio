// menus/key-ctrl-o: Ctrl+O: the effect in the window table (a window, a submenu, a changed window or the title). No menu item of the list is named as its duplicate.
import { defineScenario } from '../../lib/scenario.mjs';
import { shortcut, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/key-ctrl-o',
  covers: [],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await shortcut(ctx, 'Ctrl+O', async (g) => { await g.ctrl('KeyO', 'o', 79); }, null), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
