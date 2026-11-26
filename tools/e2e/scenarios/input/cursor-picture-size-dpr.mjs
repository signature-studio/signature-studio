// input/cursor-picture-size-dpr: the pencil picture's CSS size is its pixels divided by the device pixel ratio (at most 128 CSS px), and its hot spot is scaled the same way.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-picture-size-dpr',
  covers: ['input/cursor/picture-size-dpr'],
  run: (ctx) => I.cursorPencil(ctx, 'size'),
});
