// menus/view-19-7-choose: View > #19 > #7 (menus/item/view-19-7): the choice shows an effect: a window, a submenu, a changed window or the title. The window it opens is closed after.
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseItem, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-19-7-choose',
  covers: ['menus/item/view-19-7'],
  displays: ['4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await chooseItem(ctx, ["View","#19","#7"]), "displays: 4k-random (the crash-hunt saw this effect at DPR 1.7, none at DPR 1)");
  },
});
