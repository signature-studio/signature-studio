// menus/add-to-patterns-switch: Add open, then a click on Patterns: the first drop-down closes; a second click on Patterns opens its drop-down, to the right of Add's.
import { defineScenario } from '../../lib/scenario.mjs';
import { labelSwitch, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/add-to-patterns-switch',
  covers: ['menus/menu/patterns'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelSwitch(ctx, 'Add', 'Patterns'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
