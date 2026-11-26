// input/ctx-fl-drop-down-mixer: a right press on the Mixer shows FL's own drop-down; the browser's menu does not show (the contextmenu is cancelled).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/ctx-fl-drop-down-mixer',
  covers: ['input/menu/context-menu-fl'],
  async run(ctx) {
    await I.ensureMixer(ctx.guest);
    return I.rightClickWindow(ctx, (w) => w.cls === 'TFXForm' && w.vis !== false, 'Mixer');
  },
});
