// windows/rack-select-r3: channel 3's name is pressed: its row's strip (the selection) changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { selectChannel } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-select-r3',
  covers: ['windows/rack/select'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return selectChannel(guest, 2);
  },
});
