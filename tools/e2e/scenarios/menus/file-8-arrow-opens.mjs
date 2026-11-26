// menus/file-8-arrow-opens: File > #8 (1 items in the crash-hunt): ArrowRight on the item opens its submenu (a new drop-down).
import { defineScenario } from '../../lib/scenario.mjs';
import { arrowOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/file-8-arrow-opens',
  covers: ['menus/item/file-8'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await arrowOpens(ctx, ["File","#8"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
