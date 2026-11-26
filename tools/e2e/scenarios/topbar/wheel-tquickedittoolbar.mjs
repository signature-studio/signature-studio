// topbar/wheel-tquickedittoolbar: wheel notches: the picture of the part changes, on the top bar's part tquickedittoolbar (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-tquickedittoolbar',
  covers: ['topbar/wheel/tquickedittoolbar', 'topbar/part/tquickedittoolbar'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbar', 'wheel');
  },
});
