// topbar/click-tquickedittoolbarpanel-4: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-4 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-4',
  covers: ['topbar/click/tquickedittoolbarpanel-4', 'topbar/part/tquickedittoolbarpanel-4'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-4', 'click');
  },
});
