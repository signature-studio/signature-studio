// topbar/rclick-tquickedittoolbarpanel-9: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbarpanel-9 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbarpanel-9',
  covers: ['topbar/rclick/tquickedittoolbarpanel-9', 'topbar/part/tquickedittoolbarpanel-9'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-9', 'rclick');
  },
});
