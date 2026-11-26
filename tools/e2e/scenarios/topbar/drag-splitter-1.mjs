// topbar/drag-splitter-1: a drag along its longer side: the picture of the part changes, on the top bar's part splitter-1 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/drag-splitter-1',
  covers: ['topbar/drag/splitter-1', 'topbar/part/splitter-1'],
  expectFail: 'new', // a visible effect was not seen in the run of 2026-11-26 (a candidate for PM triage; not confirmed as a defect)
  async run({ guest }) {
    return partOperation(guest, 'splitter-1', 'drag');
  },
});
