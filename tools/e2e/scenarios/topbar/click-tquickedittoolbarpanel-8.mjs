// topbar/click-tquickedittoolbarpanel-8: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel-8 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel-8',
  covers: ['topbar/click/tquickedittoolbarpanel-8', 'topbar/part/tquickedittoolbarpanel-8'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: 'new', // no visible effect at DPR 2 in the runs of 2026-11-28 (2 runs: the rect is drawn (439 colours), the centre is the canvas); a candidate for PM triage, not confirmed as a defect
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel-8', 'click');
  },
});
