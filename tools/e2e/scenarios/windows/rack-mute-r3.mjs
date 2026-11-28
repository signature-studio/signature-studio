// windows/rack-mute-r3: the mute button of channel 3 is pressed on and off: its LED's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { muteToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-mute-r3',
  covers: ['windows/rack/mute'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return muteToggle(guest, 2);
  },
});
