// topbar/rclick-tquickedittoolbar-2: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbar-2 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbar-2',
  covers: ['topbar/rclick/tquickedittoolbar-2', 'topbar/part/tquickedittoolbar-2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbar-2', 'rclick');
  },
});
