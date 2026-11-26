// input/key-f8-plugin-list: F8 shows the plugin list (a top-level TPlugListForm window); Escape closes it after.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f8-plugin-list',
  covers: ['input/key/f8'],
  run: (ctx) => I.fkeyWindow(ctx, 8, 'TPlugListForm'),
});
