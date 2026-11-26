// windows/mixer-fader-back-4: Mixer fader 4 is raised and then lowered: the picture changes on each move.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFaderRoundTrip } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-back-4',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFaderRoundTrip(guest, 3);
  },
});
