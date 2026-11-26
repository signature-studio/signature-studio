// menus/patterns-7-esc: Patterns > #7 opens a window: it is closed by Esc (a click on the caption, then Escape), and it is gone from the window table after.
import { defineScenario } from '../../lib/scenario.mjs';
import { dialogAnswer, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/patterns-7-esc',
  covers: ['menus/item/patterns-7'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await dialogAnswer(ctx, ["Patterns","#7"], 'esc'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
