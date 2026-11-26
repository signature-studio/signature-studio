// topbar/wheel-tquickedittoolbarpanel-3: wheel notches: the picture of the part changes, on the top bar's part tquickedittoolbarpanel-3 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-tquickedittoolbarpanel-3',
  covers: ['topbar/wheel/tquickedittoolbarpanel-3', 'topbar/part/tquickedittoolbarpanel-3'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-3', 'wheel');
  },
});
