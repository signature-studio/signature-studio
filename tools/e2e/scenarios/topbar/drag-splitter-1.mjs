// topbar/drag-splitter-1: a drag along its longer side: the picture of the part changes, on the top bar's part splitter-1 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/drag-splitter-1',
  covers: ['topbar/drag/splitter-1', 'topbar/part/splitter-1'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: 'new', // no effect at DPR 2 in the runs of 2026-11-28 (the picture does not change, or changes with no menu; a candidate for PM triage, the cause is not confirmed: see the Issue #1509 report)
  async run({ guest }) {
    return partOperation(guest, 'splitter-1', 'drag');
  },
});
