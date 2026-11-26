// windows/plugin-reopen: the plugin list is closed with Escape and opened again with F8.
import { defineScenario } from '../../lib/scenario.mjs';
import { reopenPlugin } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/plugin-reopen',
  covers: ['windows/plugin/open', 'windows/plugin/close'],
  async run({ guest }) {
    return reopenPlugin(guest);
  },
});
