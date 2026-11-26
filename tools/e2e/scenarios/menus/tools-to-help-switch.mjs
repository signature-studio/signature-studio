// menus/tools-to-help-switch: Tools open, then a click on Help: the first drop-down closes; a second click on Help opens its drop-down, to the right of Tools's.
import { defineScenario } from '../../lib/scenario.mjs';
import { labelSwitch, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/tools-to-help-switch',
  covers: ['menus/menu/help'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelSwitch(ctx, 'Tools', 'Help'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
