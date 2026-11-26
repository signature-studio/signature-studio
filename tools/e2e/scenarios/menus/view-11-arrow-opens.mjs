// menus/view-11-arrow-opens: View > #11 (18 items in the crash-hunt): ArrowRight on the item opens its submenu (a new drop-down).
import { defineScenario } from '../../lib/scenario.mjs';
import { arrowOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-11-arrow-opens',
  covers: ['menus/item/view-11'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await arrowOpens(ctx, ["View","#11"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
