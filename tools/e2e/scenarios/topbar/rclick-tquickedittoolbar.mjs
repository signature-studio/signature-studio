// topbar/rclick-tquickedittoolbar: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbar (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbar',
  covers: ['topbar/rclick/tquickedittoolbar', 'topbar/part/tquickedittoolbar'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbar', 'rclick');
  },
});
