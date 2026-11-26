// menus/tools-menu-escape: Tools: Escape closes its drop-down (none is left open).
import { defineScenario } from '../../lib/scenario.mjs';
import { labelEscape, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/tools-menu-escape',
  covers: ['menus/menu/tools'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelEscape(ctx, 'Tools'), 'displays: 4k-dpr1 and 4k-random (the default)');
  },
});
