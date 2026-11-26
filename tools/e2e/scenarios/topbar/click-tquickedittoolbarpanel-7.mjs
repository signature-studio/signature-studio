// topbar/click-tquickedittoolbarpanel-7: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-7 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-7',
  covers: ['topbar/click/tquickedittoolbarpanel-7', 'topbar/part/tquickedittoolbarpanel-7'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-7', 'click');
  },
});
