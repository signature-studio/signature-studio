// topbar/wheel-tquickedittoolbarpanel-9: wheel notches: the picture of the part changes, on the top bar's part tquickedittoolbarpanel-9 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-tquickedittoolbarpanel-9',
  covers: ['topbar/wheel/tquickedittoolbarpanel-9', 'topbar/part/tquickedittoolbarpanel-9'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-9', 'wheel');
  },
});
