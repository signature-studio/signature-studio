// menus/key-ctrl-n: Ctrl+N and File > #4 (menus/item/file-4): the two paths give the same kind of effect (a window, a submenu, a changed window or the title).
import { defineScenario } from '../../lib/scenario.mjs';
import { keyAndMenu, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/key-ctrl-n',
  covers: ['menus/item/file-4'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await keyAndMenu(ctx, 'Ctrl+N', async (g) => { await g.ctrl('KeyN', 'n', 78); }, ["File","#4"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
