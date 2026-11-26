// topbar/click-tquickedittoolbar-2: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbar-2 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbar-2',
  covers: ['topbar/click/tquickedittoolbar-2', 'topbar/part/tquickedittoolbar-2'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbar-2', 'click');
  },
});
