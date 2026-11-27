// windows/step-r5-c05: the Channel rack's step (row 5, step 5) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r5-c05',
  covers: ['windows/step/toggle'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return stepToggle(guest, 4, 4);
  },
});
