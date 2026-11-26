// menus/patterns-to-view-switch: Patterns open, then a click on View: the first drop-down closes; a second click on View opens its drop-down, to the right of Patterns's.
import { defineScenario } from '../../lib/scenario.mjs';
import { labelSwitch, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/patterns-to-view-switch',
  covers: ['menus/menu/view'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelSwitch(ctx, 'Patterns', 'View'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
