// topbar/click-twpcontrol-2: a left click: the picture of the part changes, or a window opens, on the top bar's part twpcontrol-2 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-twpcontrol-2',
  covers: ['topbar/click/twpcontrol-2', 'topbar/part/twpcontrol-2'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol-2', 'click');
  },
});
