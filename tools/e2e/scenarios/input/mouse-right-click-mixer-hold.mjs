// input/mouse-right-click-mixer-hold: a right press held for 800 ms on the Mixer still opens its drop-down.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-right-click-mixer-hold',
  covers: ['input/mouse/right-click'],
  async run(ctx) {
    await I.ensureMixer(ctx.guest);
    return I.rightClickWindow(ctx, (w) => w.cls === 'TFXForm' && w.vis !== false, 'Mixer (held)', 800);
  },
});
