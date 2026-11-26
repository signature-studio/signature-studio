// menus/key-ctrl-e: Ctrl+E: the effect in the window table (a window, a submenu, a changed window or the title). No menu item of the list is named as its duplicate.
import { defineScenario } from '../../lib/scenario.mjs';
import { shortcut, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/key-ctrl-e',
  covers: [],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await shortcut(ctx, 'Ctrl+E', async (g) => { await g.ctrl('KeyE', 'e', 69); }, null), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
