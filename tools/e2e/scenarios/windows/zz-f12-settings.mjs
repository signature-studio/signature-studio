// windows/zz-f12-settings: F12 closes the Settings window (the table keeps only the main form). F12 also hides the Channel rack, the Playlist and the Mixer for the rest of the page, so this runs last in its shard (the zz- prefix).
import { defineScenario } from '../../lib/scenario.mjs';
import { f12CloseAll } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/zz-f12-settings',
  covers: ['windows/op/f12'],
  async run({ guest }) {
    return f12CloseAll(guest, 'F10');
  },
});
