// windows/piano-close: the Piano roll opened by F7 is gone from the table after Escape.
import { defineScenario } from '../../lib/scenario.mjs';
import { pianoRoll } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/piano-close',
  covers: ['windows/piano/close'],
  async run({ guest }) {
    return pianoRoll(guest, true);
  },
});
