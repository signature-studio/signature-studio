// windows/mixer-fader-back-5: Mixer fader 5 is raised and then lowered: the picture changes on each move.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFaderRoundTrip } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-back-5',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFaderRoundTrip(guest, 4);
  },
});
