// windows/mixer-fader-up-3: Mixer fader 3 is dragged up by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-up-3',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFader(guest, 2, 40);
  },
});
