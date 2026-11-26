// page/dpr-change-down-1-5: the page moves to a monitor of DPR 1.5 while it runs: the canvas is crisp at 1.5 and the picture keeps drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { dprMoveCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/dpr-change-down-1-5',
  covers: ['page/dpr-change/down-1-5'],
  async run(ctx) {
    return dprMoveCase(ctx, 1.5);
  },
});
