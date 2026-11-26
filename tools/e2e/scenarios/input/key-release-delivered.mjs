// input/key-release-delivered: a key goes down and up (F12): both reach the engine.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-release-delivered',
  covers: ['input/key/release'],
  run: (ctx) => I.releaseCase(ctx),
});
