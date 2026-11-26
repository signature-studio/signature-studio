// input/mouse-middle-click-mixer: a middle press and release at the Mixer's centre reach the engine as the middle button, and the Mixer takes the hit.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-middle-click-mixer',
  covers: ['input/mouse/middle-click'],
  run: (ctx) => I.clickTrace(ctx, 'middle', 'mixer'),
});
