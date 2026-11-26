// topbar/click-tquickedittoolbarpanel-8: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-8 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-8',
  covers: ['topbar/click/tquickedittoolbarpanel-8', 'topbar/part/tquickedittoolbarpanel-8'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-8', 'click');
  },
});
