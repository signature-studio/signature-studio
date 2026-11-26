// input/key-escape-closes-plugin-list: Escape closes the plugin list that F8 shows.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-escape-closes-plugin-list',
  covers: ['input/key/escape'],
  run: (ctx) => I.escapeWindow(ctx, 8, 'TPlugListForm'),
});
