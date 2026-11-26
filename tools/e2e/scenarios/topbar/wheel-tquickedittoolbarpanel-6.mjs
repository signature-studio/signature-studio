// topbar/wheel-tquickedittoolbarpanel-6: wheel notches: the picture of the part changes, on the top bar's part tquickedittoolbarpanel-6 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-tquickedittoolbarpanel-6',
  covers: ['topbar/wheel/tquickedittoolbarpanel-6', 'topbar/part/tquickedittoolbarpanel-6'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-6', 'wheel');
  },
});
