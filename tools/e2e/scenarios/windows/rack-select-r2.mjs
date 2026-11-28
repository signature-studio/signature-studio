// windows/rack-select-r2: channel 2's name is pressed: its row's strip (the selection) changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { selectChannel } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-select-r2',
  covers: ['windows/rack/select'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return selectChannel(guest, 1);
  },
});
