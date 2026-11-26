// input/touch-pinch-random-dpr: two fingers at a random device pixel ratio: only the first finger reaches the engine.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-pinch-random-dpr',
  covers: ['input/touch/pinch'],
  displays: ['4k-random'],
  run: (ctx) => I.touchPinch(ctx),
});
