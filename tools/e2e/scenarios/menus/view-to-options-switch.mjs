// menus/view-to-options-switch: View open, then a click on Options: the first drop-down closes; a second click on Options opens its drop-down, to the right of View's.
import { defineScenario } from '../../lib/scenario.mjs';
import { labelSwitch, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-to-options-switch',
  covers: ['menus/menu/options'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelSwitch(ctx, 'View', 'Options'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
