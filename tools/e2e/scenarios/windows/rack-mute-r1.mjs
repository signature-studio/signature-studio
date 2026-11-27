// windows/rack-mute-r1: the mute button of channel 1 is pressed on and off: its LED's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { muteToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-mute-r1',
  covers: ['windows/rack/mute'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return muteToggle(guest, 0);
  },
});
