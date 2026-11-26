// windows/rack-mute-r5: the mute button of channel 5 is pressed on and off: its LED's picture changes each time.
import { defineScenario } from '../../lib/scenario.mjs';
import { muteToggle } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-mute-r5',
  covers: ['windows/rack/mute'],
  async run({ guest }) {
    return muteToggle(guest, 4);
  },
});
