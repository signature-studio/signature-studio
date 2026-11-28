// topbar/click-tquickedittoolbarpanel-6: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-6 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-6',
  covers: ['topbar/click/tquickedittoolbarpanel-6', 'topbar/part/tquickedittoolbarpanel-6'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-6', 'click');
  },
});
