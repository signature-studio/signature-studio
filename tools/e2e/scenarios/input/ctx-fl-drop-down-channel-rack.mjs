// input/ctx-fl-drop-down-channel-rack: a right press on the Channel rack shows FL's own drop-down.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/ctx-fl-drop-down-channel-rack',
  covers: ['input/menu/context-menu-fl'],
  async run(ctx) {
    await I.ensureChannelRack(ctx.guest);
    return I.rightClickWindow(ctx, (w) => w.cls === 'TStepSeqForm' && w.vis !== false, 'Channel rack');
  },
});
