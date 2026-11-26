// topbar/wheel-tquickedittoolbarpanel-5: wheel notches: the picture of the part changes, on the top bar's part tquickedittoolbarpanel-5 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-tquickedittoolbarpanel-5',
  covers: ['topbar/wheel/tquickedittoolbarpanel-5', 'topbar/part/tquickedittoolbarpanel-5'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-5', 'wheel');
  },
});
