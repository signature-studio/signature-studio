// windows/mixer-fader-down-1: Mixer fader 1 is dragged down by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-down-1',
  covers: ['windows/mixer/fader'],
  async run({ guest }) {
    return mixerFader(guest, 0, -40);
  },
});
