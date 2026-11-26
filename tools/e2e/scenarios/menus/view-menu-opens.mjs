// menus/view-menu-opens: View: a click on the label opens its drop-down (its item count is read from the E0 list).
import { defineScenario } from '../../lib/scenario.mjs';
import { labelOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-menu-opens',
  covers: ['menus/menu/view'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelOpens(ctx, 'View'), 'displays: 4k-dpr1 and 4k-random (the default)');
  },
});
