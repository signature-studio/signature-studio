// topbar/click-tnewmenu: a left click: the picture of the part changes, or a window opens, on the top bar's part tnewmenu (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tnewmenu',
  covers: ['topbar/click/tnewmenu', 'topbar/part/tnewmenu'],
  async run({ guest }) {
    return partOperation(guest, 'tnewmenu', 'click');
  },
});
