// windows/mixer-fader-up-6: Mixer fader 6 is dragged up by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-up-6',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFader(guest, 5, 40);
  },
});
