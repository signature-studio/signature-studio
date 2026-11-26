// topbar/rclick-tquickedittoolbarpanel: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part tquickedittoolbarpanel (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-tquickedittoolbarpanel',
  covers: ['topbar/rclick/tquickedittoolbarpanel', 'topbar/part/tquickedittoolbarpanel'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel', 'rclick');
  },
});
