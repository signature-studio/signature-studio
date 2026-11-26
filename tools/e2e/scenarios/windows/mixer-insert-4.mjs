// windows/mixer-insert-4: Mixer insert 4 is pressed: the insert area's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerInsert } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-insert-4',
  covers: ['windows/mixer/insert-select'],
  async run({ guest }) {
    return mixerInsert(guest, 3);
  },
});
