// windows/mixer-fader-down-3: Mixer fader 3 is dragged down by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-down-3',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFader(guest, 2, -40);
  },
});
