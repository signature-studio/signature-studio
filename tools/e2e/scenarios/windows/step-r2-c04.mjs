// windows/step-r2-c04: the Channel rack's step (row 2, step 4) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r2-c04',
  covers: ['windows/step/toggle'],
  async run({ guest }) {
    return stepToggle(guest, 1, 3);
  },
});
