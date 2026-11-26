// menus/add-6-cancel: Add > #6 opens a window: it is closed by Cancel (Escape), and it is gone from the window table after.
import { defineScenario } from '../../lib/scenario.mjs';
import { dialogAnswer, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/add-6-cancel',
  covers: ['menus/item/add-6'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await dialogAnswer(ctx, ["Add","#6"], 'cancel'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
