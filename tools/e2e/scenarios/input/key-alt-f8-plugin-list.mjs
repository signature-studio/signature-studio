// input/key-alt-f8-plugin-list: Alt+F8 opens the plugin list (a top-level TPlugListForm window). Not met: both keys reach the engine and the list does not show (F8 alone shows it).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-alt-f8-plugin-list',
  covers: ['input/key/alt'],
  expectFail: 'new',
  run: (ctx) => I.altF8(ctx),
});
