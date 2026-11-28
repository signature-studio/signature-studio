// windows/step-r5-c01: the Channel rack's step (row 5, step 1) is pressed on, then off: its row's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { stepToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/step-r5-c01',
  covers: ['windows/step/toggle'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return stepToggle(guest, 4, 0);
  },
});
