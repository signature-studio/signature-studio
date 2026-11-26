// menus/file-9-9-arrow-opens: File > #9 > #9 (2 items in the crash-hunt): ArrowRight on the item opens its submenu (a new drop-down).
import { defineScenario } from '../../lib/scenario.mjs';
import { arrowOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/file-9-9-arrow-opens',
  covers: ['menus/item/file-9-9'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await arrowOpens(ctx, ["File","#9","#9"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
