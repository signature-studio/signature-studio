// input/cursor-picture-size-dpr-1: the pencil picture at device pixel ratio 1: the CSS size is the picture's own pixels (at most 128).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-picture-size-dpr-1',
  covers: ['input/cursor/picture-size-dpr'],
  displays: ['4k-dpr1'],
  run: (ctx) => I.cursorPencil(ctx, 'size'),
});
