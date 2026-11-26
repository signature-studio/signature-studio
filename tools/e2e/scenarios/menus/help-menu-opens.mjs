// menus/help-menu-opens: Help: a click on the label opens its drop-down (its item count is read from the E0 list).
import { defineScenario } from '../../lib/scenario.mjs';
import { labelOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/help-menu-opens',
  covers: ['menus/menu/help'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelOpens(ctx, 'Help'), 'displays: 4k-dpr1 and 4k-random (the default)');
  },
});
