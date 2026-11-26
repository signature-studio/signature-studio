// input/mouse-right-click-mixer: a right press on the Mixer opens its drop-down (FL's own menu) and Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-right-click-mixer',
  covers: ['input/mouse/right-click'],
  async run(ctx) {
    await I.ensureMixer(ctx.guest);
    return I.rightClickWindow(ctx, (w) => w.cls === 'TFXForm' && w.vis !== false, 'Mixer');
  },
});
