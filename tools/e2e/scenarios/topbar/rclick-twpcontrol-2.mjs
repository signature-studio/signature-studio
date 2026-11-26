// topbar/rclick-twpcontrol-2: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part twpcontrol-2 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-twpcontrol-2',
  covers: ['topbar/rclick/twpcontrol-2', 'topbar/part/twpcontrol-2'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol-2', 'rclick');
  },
});
