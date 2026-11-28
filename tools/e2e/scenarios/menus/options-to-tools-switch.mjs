// menus/options-to-tools-switch: Options open, then a click on Tools: the first drop-down closes; a second click on Tools opens its drop-down, to the right of Options's.
import { defineScenario } from '../../lib/scenario.mjs';
import { labelSwitch, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/options-to-tools-switch',
  covers: ['menus/menu/tools'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelSwitch(ctx, 'Options', 'Tools'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
