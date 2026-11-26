// topbar/drag-twpcontrol: a drag along its longer side: the picture of the part changes, on the top bar's part twpcontrol (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/drag-twpcontrol',
  covers: ['topbar/drag/twpcontrol', 'topbar/part/twpcontrol'],
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol', 'drag');
  },
});
