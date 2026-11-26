// windows/step-r5-c06: the Channel rack's step (row 5, step 6) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r5-c06',
  covers: ['windows/step/toggle'],
  async run({ guest }) {
    return stepToggle(guest, 4, 5);
  },
});
