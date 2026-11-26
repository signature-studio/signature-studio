// page/dpr-change-keeps-drawing: after a change of the ratio (to 1.75) the engine keeps drawing: the File menu opens and closes as a change of the picture, and the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { dprMoveCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/dpr-change-keeps-drawing',
  covers: ['page/dpr-change/keeps-drawing'],
  async run(ctx) {
    return dprMoveCase(ctx, 1.75);
  },
});
