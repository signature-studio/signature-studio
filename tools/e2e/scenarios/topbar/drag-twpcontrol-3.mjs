// topbar/drag-twpcontrol-3: a drag along its longer side: the picture of the part changes, on the top bar's part twpcontrol-3 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/drag-twpcontrol-3',
  covers: ['topbar/drag/twpcontrol-3', 'topbar/part/twpcontrol-3'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol-3', 'drag');
  },
});
