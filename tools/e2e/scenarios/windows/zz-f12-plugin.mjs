// windows/zz-f12-plugin: F12 closes the plugin list (the table keeps only the main form). Known defect: the plugin list stays open after F12, and F12 hides the Channel rack, the Playlist and the Mixer (they do not come back with a second F12); it runs last in its shard (the zz- prefix).
import { defineScenario } from '../../lib/scenario.mjs';
import { f12CloseAll } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/zz-f12-plugin',
  covers: ['windows/op/f12'],
  expectFail: 'new',
  async run({ guest }) {
    return f12CloseAll(guest, 'F8');
  },
});
