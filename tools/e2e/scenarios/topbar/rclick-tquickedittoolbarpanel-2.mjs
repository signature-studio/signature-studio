// topbar/rclick-tquickedittoolbarpanel-2: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbarpanel-2 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbarpanel-2',
  covers: ['topbar/rclick/tquickedittoolbarpanel-2', 'topbar/part/tquickedittoolbarpanel-2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-2', 'rclick');
  },
});
