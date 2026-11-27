// windows/zz-mixer-insert-7: Mixer insert 7 is pressed. Known defect: after the press the Channel rack is hidden in the table for the rest of the page (vis 0; the picture of the insert is not checked, the scenario is a finding). It runs last in its shard (the zz- prefix).
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerInsert } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/zz-mixer-insert-7',
  covers: ['windows/mixer/insert-select'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: 'new',
  async run({ guest }) {
    return mixerInsert(guest, 6);
  },
});
