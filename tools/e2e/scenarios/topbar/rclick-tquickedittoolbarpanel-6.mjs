// topbar/rclick-tquickedittoolbarpanel-6: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbarpanel-6 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbarpanel-6',
  covers: ['topbar/rclick/tquickedittoolbarpanel-6', 'topbar/part/tquickedittoolbarpanel-6'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-6', 'rclick');
  },
});
