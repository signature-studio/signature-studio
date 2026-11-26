// topbar/click-twpcontrol-3: a left click: the picture of the part changes, or a window opens, on the top bar's part twpcontrol-3 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-twpcontrol-3',
  covers: ['topbar/click/twpcontrol-3', 'topbar/part/twpcontrol-3'],
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol-3', 'click');
  },
});
