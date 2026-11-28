// windows/mixer-fader-down-8: Mixer fader 8 is dragged down by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-down-8',
  covers: ['windows/mixer/fader'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return mixerFader(guest, 7, -40);
  },
});
