// menus/file-6-esc: File > #6 opens a window: it is closed by Esc (a click on the caption, then Escape), and it is gone from the window table after.
import { defineScenario } from '../../lib/scenario.mjs';
import { dialogAnswer, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/file-6-esc',
  covers: ['menus/item/file-6'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await dialogAnswer(ctx, ["File","#6"], 'esc'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
