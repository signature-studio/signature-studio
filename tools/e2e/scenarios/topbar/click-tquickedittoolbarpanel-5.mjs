// topbar/click-tquickedittoolbarpanel-5: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-5 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-5',
  covers: ['topbar/click/tquickedittoolbarpanel-5', 'topbar/part/tquickedittoolbarpanel-5'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-5', 'click');
  },
});
