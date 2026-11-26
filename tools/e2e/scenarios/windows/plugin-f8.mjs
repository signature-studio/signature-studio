// windows/plugin-f8: F8 opens the plugin list (TPlugListForm, top-level).
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginListOpen } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/plugin-f8',
  covers: ['windows/window/tpluglistform', 'windows/plugin/open'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return pluginListOpen(guest, 'F8');
  },
});
