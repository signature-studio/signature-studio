// windows/mixer-insert-6: Mixer insert 6 is pressed: the insert area's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerInsert } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-insert-6',
  covers: ['windows/mixer/insert-select'],
  async run({ guest }) {
    return mixerInsert(guest, 5);
  },
});
