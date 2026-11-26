// windows/step-r1-c09: the Channel rack's step (row 1, step 9) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r1-c09',
  covers: ['windows/step/toggle'],
  async run({ guest }) {
    return stepToggle(guest, 0, 8);
  },
});
