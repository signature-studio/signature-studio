// input/touch-pinch-first-finger-only: two fingers on the desk: only the first finger reaches the engine (the second is not the primary pointer).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-pinch-first-finger-only',
  covers: ['input/touch/pinch'],
  run: (ctx) => I.touchPinch(ctx),
});
