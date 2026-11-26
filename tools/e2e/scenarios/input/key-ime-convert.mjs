// input/key-ime-convert: an IME conversion (the composition changed to the converted text) should reach the box. The page does not forward a composition: a known gap, recorded.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-ime-convert',
  covers: ['input/key/ime-convert'],
  expectFail: 'new',
  run: (ctx) => I.imeCase(ctx, 'convert'),
});
