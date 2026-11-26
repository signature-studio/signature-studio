// topbar/click-tquickedittoolbarpanel-11: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-11 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-11',
  covers: ['topbar/click/tquickedittoolbarpanel-11', 'topbar/part/tquickedittoolbarpanel-11'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-11', 'click');
  },
});
