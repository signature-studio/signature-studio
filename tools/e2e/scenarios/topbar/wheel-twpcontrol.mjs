// topbar/wheel-twpcontrol: wheel notches: the picture of the part changes, on the top bar's part twpcontrol (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-twpcontrol',
  covers: ['topbar/wheel/twpcontrol', 'topbar/part/twpcontrol'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol', 'wheel');
  },
});
