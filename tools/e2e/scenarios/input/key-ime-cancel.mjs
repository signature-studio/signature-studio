// input/key-ime-cancel: a cancelled IME composition leaves no text in the box. Not met: the composed text does not reach the guest's memory during the composition (the same as key-ime-compose), so the cancel is not measured.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-ime-cancel',
  covers: ['input/key/ime-cancel'],
  expectFail: 'new',
  run: (ctx) => I.imeCase(ctx, 'cancel'),
});
