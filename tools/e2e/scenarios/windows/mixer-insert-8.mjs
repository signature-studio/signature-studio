// windows/mixer-insert-8: Mixer insert 8 is pressed: the insert area's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerInsert } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-insert-8',
  covers: ['windows/mixer/insert-select'],
  async run({ guest }) {
    return mixerInsert(guest, 7);
  },
});
