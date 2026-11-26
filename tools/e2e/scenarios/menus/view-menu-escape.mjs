// menus/view-menu-escape: View: Escape closes its drop-down (none is left open).
import { defineScenario } from '../../lib/scenario.mjs';
import { labelEscape, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/view-menu-escape',
  covers: ['menus/menu/view'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelEscape(ctx, 'View'), 'displays: 4k-dpr1 and 4k-random (the default)');
  },
});
