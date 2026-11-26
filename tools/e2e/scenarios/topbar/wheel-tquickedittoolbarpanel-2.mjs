// topbar/wheel-tquickedittoolbarpanel-2: wheel notches: the picture of the part changes, on the top bar's part tquickedittoolbarpanel-2 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-tquickedittoolbarpanel-2',
  covers: ['topbar/wheel/tquickedittoolbarpanel-2', 'topbar/part/tquickedittoolbarpanel-2'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-2', 'wheel');
  },
});
