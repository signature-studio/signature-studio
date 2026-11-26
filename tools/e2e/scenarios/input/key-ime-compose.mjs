// input/key-ime-compose: an IME composition (Japanese, Input.imeSetComposition) on the channel name box should reach the box. The page does not forward a composition (keymap.ts keyPolicy: isComposing is not forwarded): a known gap, recorded.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-ime-compose',
  covers: ['input/key/ime-compose'],
  expectFail: 'new',
  run: (ctx) => I.imeCase(ctx, 'compose'),
});
