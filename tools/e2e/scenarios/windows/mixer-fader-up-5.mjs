// windows/mixer-fader-up-5: Mixer fader 5 is dragged up by 40 px: its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { mixerFader } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/mixer-fader-up-5',
  covers: ['windows/mixer/fader'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return mixerFader(guest, 4, 40);
  },
});
