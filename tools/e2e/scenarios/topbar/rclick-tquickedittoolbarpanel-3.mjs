// topbar/rclick-tquickedittoolbarpanel-3: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbarpanel-3 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbarpanel-3',
  covers: ['topbar/rclick/tquickedittoolbarpanel-3', 'topbar/part/tquickedittoolbarpanel-3'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-3', 'rclick');
  },
});
