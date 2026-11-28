// topbar/click-tquickedittoolbarpanel: a left click: the picture of the part changes, or a window opens, on the top bar's part tquickedittoolbarpanel (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-tquickedittoolbarpanel',
  covers: ['topbar/click/tquickedittoolbarpanel', 'topbar/part/tquickedittoolbarpanel'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: 'new', // no effect at DPR 2 in the runs of 2026-11-28 (the picture does not change, or changes with no menu; a candidate for PM triage, the cause is not confirmed: see the Issue #1509 report)
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbarpanel', 'click');
  },
});
