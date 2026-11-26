// topbar/click-tquickedittoolbarpanel-12: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-12 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-12',
  covers: ['topbar/click/tquickedittoolbarpanel-12', 'topbar/part/tquickedittoolbarpanel-12'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-12', 'click');
  },
});
