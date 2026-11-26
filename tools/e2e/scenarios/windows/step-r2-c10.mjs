// windows/step-r2-c10: the Channel rack's step (row 2, step 10) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r2-c10',
  covers: ['windows/step/toggle'],
  async run({ guest }) {
    return stepToggle(guest, 1, 9);
  },
});
