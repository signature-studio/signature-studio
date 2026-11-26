// menus/edit-to-add-switch: Edit open, then a click on Add: the first drop-down closes; a second click on Add opens its drop-down, to the right of Edit's.
import { defineScenario } from '../../lib/scenario.mjs';
import { labelSwitch, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/edit-to-add-switch',
  covers: ['menus/menu/add'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelSwitch(ctx, 'Edit', 'Add'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
