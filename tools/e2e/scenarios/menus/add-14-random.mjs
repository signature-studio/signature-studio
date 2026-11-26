// menus/add-14-random: Add > #14 (menus/item/add-14) at DPR 1.7 (the random display): the choice shows its window (the same item stops the engine at DPR 1: add-14-choose).
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseItem, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/add-14-random',
  covers: ['menus/item/add-14'],
  displays: ['4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await chooseItem(ctx, ["Add","#14"]), "displays: 4k-random (the crash-hunt saw this effect at DPR 1.7, none at DPR 1)");
  },
});
