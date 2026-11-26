// windows/mixer-fader-down-8: Mixer fader 8 is dragged down by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-down-8',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFader(guest, 7, -40);
  },
});
