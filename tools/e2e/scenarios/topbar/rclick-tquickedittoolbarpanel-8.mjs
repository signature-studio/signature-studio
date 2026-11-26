// topbar/rclick-tquickedittoolbarpanel-8: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbarpanel-8 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbarpanel-8',
  covers: ['topbar/rclick/tquickedittoolbarpanel-8', 'topbar/part/tquickedittoolbarpanel-8'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-8', 'rclick');
  },
});
