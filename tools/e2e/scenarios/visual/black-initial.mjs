// visual/black-initial: at the start, no all-black 64 x 64 tile is drawn.
import { defineScenario } from '../../lib/scenario.mjs';
import { blackInitial } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/black-initial',
  covers: ['visual/black/initial'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => blackInitial(ctx),
});
