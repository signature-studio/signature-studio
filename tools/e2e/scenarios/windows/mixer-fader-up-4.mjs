// windows/mixer-fader-up-4: Mixer fader 4 is dragged up by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-up-4',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFader(guest, 3, 40);
  },
});
