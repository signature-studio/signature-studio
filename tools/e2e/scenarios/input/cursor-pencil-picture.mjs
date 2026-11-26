// input/cursor-pencil-picture: the Piano roll's pencil (Draw) is a picture cursor (image-set) at the page's device pixel ratio, with the hot spot in CSS px.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-pencil-picture',
  covers: ['input/cursor/pencil-picture'],
  run: (ctx) => I.cursorPencil(ctx, 'css'),
});
