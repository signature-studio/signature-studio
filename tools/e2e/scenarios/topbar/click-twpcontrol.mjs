// topbar/click-twpcontrol: a left click: the picture of the part changes, or a window opens, on the top bar's part twpcontrol (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-twpcontrol',
  covers: ['topbar/click/twpcontrol', 'topbar/part/twpcontrol'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol', 'click');
  },
});
