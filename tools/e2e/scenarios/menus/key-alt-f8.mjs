// menus/key-alt-f8: Alt+F8: the effect in the window table (a window, a submenu, a changed window or the title). No menu item of the list is named as its duplicate.
import { defineScenario } from '../../lib/scenario.mjs';
import { altChord, shortcut, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/key-alt-f8',
  covers: [],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await shortcut(ctx, 'Alt+F8', async (g) => { await altChord(g, 'F8', 'F8', 119); }, null), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
