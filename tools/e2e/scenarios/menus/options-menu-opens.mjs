// menus/options-menu-opens: Options: a click on the label opens its drop-down (its item count is read from the E0 list).
import { defineScenario } from '../../lib/scenario.mjs';
import { labelOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/options-menu-opens',
  covers: ['menus/menu/options'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelOpens(ctx, 'Options'), 'displays: 4k-dpr1 and 4k-random (the default)');
  },
});
