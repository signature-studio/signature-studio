// menus/options-menu-escape: Options: Escape closes its drop-down (none is left open).
import { defineScenario } from '../../lib/scenario.mjs';
import { labelEscape, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/options-menu-escape',
  covers: ['menus/menu/options'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: null,
  async run(ctx) {
    return withNote(await labelEscape(ctx, 'Options'), 'displays: 4k-dpr1 and 4k-random (the default)');
  },
});
