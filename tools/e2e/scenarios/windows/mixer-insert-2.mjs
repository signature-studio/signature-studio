// windows/mixer-insert-2: Mixer insert 2 is pressed: the insert area's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerInsert } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-insert-2',
  covers: ['windows/mixer/insert-select'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return mixerInsert(guest, 1);
  },
});
