// topbar/wheel-twpcontrol-3: wheel notches: the picture of the part changes, on the top bar's part twpcontrol-3 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-twpcontrol-3',
  covers: ['topbar/wheel/twpcontrol-3', 'topbar/part/twpcontrol-3'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol-3', 'wheel');
  },
});
