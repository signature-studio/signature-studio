// menus/key-ctrl-i: Ctrl+I: the effect in the window table (a window, a submenu, a changed window or the title). No menu item of the list is named as its duplicate.
import { defineScenario } from '../../lib/scenario.mjs';
import { shortcut, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/key-ctrl-i',
  covers: [],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await shortcut(ctx, 'Ctrl+I', async (g) => { await g.ctrl('KeyI', 'i', 73); }, null), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
