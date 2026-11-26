// windows/step-r3-c01: the Channel rack's step (row 3, step 1) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r3-c01',
  covers: ['windows/step/toggle'],
  async run({ guest }) {
    return stepToggle(guest, 2, 0);
  },
});
