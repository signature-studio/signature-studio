// windows/step-r4-c08: the Channel rack's step (row 4, step 8) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r4-c08',
  covers: ['windows/step/toggle'],
  async run({ guest }) {
    return stepToggle(guest, 3, 7);
  },
});
