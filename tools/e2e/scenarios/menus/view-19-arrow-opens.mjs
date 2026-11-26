// menus/view-19-arrow-opens: View > #19 (7 items in the crash-hunt): ArrowRight on the item opens its submenu (a new drop-down).
import { defineScenario } from '../../lib/scenario.mjs';
import { arrowOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-19-arrow-opens',
  covers: ['menus/item/view-19'],
  displays: ['4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await arrowOpens(ctx, ["View","#19"]), "displays: 4k-random (the crash-hunt saw this effect at DPR 1.7, none at DPR 1)");
  },
});
