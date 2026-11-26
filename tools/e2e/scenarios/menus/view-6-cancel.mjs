// menus/view-6-cancel: View > #6 opens a window: it is closed by Cancel (Escape), and it is gone from the window table after.
import { defineScenario } from '../../lib/scenario.mjs';
import { dialogAnswer, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-6-cancel',
  covers: ['menus/item/view-6'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await dialogAnswer(ctx, ["View","#6"], 'cancel'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
