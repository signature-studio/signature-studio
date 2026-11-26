// menus/patterns-menu-escape: Patterns: Escape closes its drop-down (none is left open).
import { defineScenario } from '../../lib/scenario.mjs';
import { labelEscape, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/patterns-menu-escape',
  covers: ['menus/menu/patterns'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelEscape(ctx, 'Patterns'), 'displays: 4k-dpr1 and 4k-random (the default)');
  },
});
