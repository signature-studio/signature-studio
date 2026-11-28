// topbar/rclick-splitter-1: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part splitter-1 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-splitter-1',
  covers: ['topbar/rclick/splitter-1', 'topbar/part/splitter-1'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'splitter-1', 'rclick');
  },
});
