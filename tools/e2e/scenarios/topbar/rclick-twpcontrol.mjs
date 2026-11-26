// topbar/rclick-twpcontrol: a right click: a drop-down menu (TQuickPopupMenuWindow) opens at the point, on the top bar's part twpcontrol (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/rclick-twpcontrol',
  covers: ['topbar/rclick/twpcontrol', 'topbar/part/twpcontrol'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'twpcontrol', 'rclick');
  },
});
