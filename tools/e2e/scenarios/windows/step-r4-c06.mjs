// windows/step-r4-c06: the Channel rack's step (row 4, step 6) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r4-c06',
  covers: ['windows/step/toggle'],
  async run({ guest }) {
    return stepToggle(guest, 3, 5);
  },
});
