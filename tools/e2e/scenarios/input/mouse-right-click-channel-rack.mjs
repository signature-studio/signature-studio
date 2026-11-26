// input/mouse-right-click-channel-rack: a right press on the Channel rack opens its drop-down.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-right-click-channel-rack',
  covers: ['input/mouse/right-click'],
  async run(ctx) {
    await I.ensureChannelRack(ctx.guest);
    return I.rightClickWindow(ctx, (w) => w.cls === 'TStepSeqForm' && w.vis !== false, 'Channel rack');
  },
});
