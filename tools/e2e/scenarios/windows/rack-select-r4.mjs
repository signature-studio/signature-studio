// windows/rack-select-r4: channel 4's name is pressed: its row's strip (the selection) changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { selectChannel } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-select-r4',
  covers: ['windows/rack/select'],
  async run({ guest }) {
    return selectChannel(guest, 3);
  },
});
