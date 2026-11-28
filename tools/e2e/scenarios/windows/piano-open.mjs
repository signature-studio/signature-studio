// windows/piano-open: with channel 1 selected, F7 opens the Piano roll (a TEventEditForm child, caption "Piano roll"); Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import { pianoRoll } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/piano-open',
  covers: ['windows/piano/open', 'windows/event/open'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return pianoRoll(guest, true);
  },
});
