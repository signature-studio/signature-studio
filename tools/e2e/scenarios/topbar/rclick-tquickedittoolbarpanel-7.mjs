// topbar/rclick-tquickedittoolbarpanel-7: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbarpanel-7 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbarpanel-7',
  covers: ['topbar/rclick/tquickedittoolbarpanel-7', 'topbar/part/tquickedittoolbarpanel-7'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-7', 'rclick');
  },
});
