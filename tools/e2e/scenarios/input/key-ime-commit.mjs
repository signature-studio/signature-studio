// input/key-ime-commit: a committed IME text (Input.insertText) should reach the box. The page sends no committed text to the engine: a known gap, recorded.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-ime-commit',
  covers: ['input/key/ime-commit'],
  expectFail: 'new',
  run: (ctx) => I.imeCase(ctx, 'commit'),
});
