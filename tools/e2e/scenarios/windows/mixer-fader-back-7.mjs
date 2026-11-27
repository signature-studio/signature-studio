// windows/mixer-fader-back-7: Mixer fader 7 is raised and then lowered: the picture changes on each move.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFaderRoundTrip } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-back-7',
  covers: ['windows/mixer/fader'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return mixerFaderRoundTrip(guest, 6);
  },
});
