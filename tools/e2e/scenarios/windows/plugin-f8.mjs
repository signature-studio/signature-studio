// windows/plugin-f8: F8 opens the plugin list (TPlugListForm, top-level).
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginListOpen } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/plugin-f8',
  covers: ['windows/window/tpluglistform', 'windows/plugin/open'],
  async run({ guest }) {
    return pluginListOpen(guest, 'F8');
  },
});
