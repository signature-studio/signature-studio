// menus/file-to-edit-switch: File open, then a click on Edit: the first drop-down closes; a second click on Edit opens its drop-down, to the right of File's.
import { defineScenario } from '../../lib/scenario.mjs';
import { labelSwitch, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/file-to-edit-switch',
  covers: ['menus/menu/edit'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelSwitch(ctx, 'File', 'Edit'), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
