// input/cursor-pencil-picture-random-dpr: the pencil picture cursor at a random device pixel ratio: the image-set scale is that ratio.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-pencil-picture-random-dpr',
  covers: ['input/cursor/pencil-picture'],
  displays: ['4k-random'],
  run: (ctx) => I.cursorPencil(ctx, 'css'),
});
