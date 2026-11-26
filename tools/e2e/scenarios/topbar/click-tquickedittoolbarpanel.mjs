// topbar/click-tquickedittoolbarpanel: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel',
  covers: ['topbar/click/tquickedittoolbarpanel', 'topbar/part/tquickedittoolbarpanel'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel', 'click');
  },
});
