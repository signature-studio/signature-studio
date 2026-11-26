// page/dpr-change-up-2: the page moves to a monitor of DPR 2 while it runs (the window keeps its physical size): the canvas is crisp at 2 and the picture keeps drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { dprMoveCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/dpr-change-up-2',
  covers: ['page/dpr-change/up-2'],
  async run(ctx) {
    return dprMoveCase(ctx, 2);
  },
});
