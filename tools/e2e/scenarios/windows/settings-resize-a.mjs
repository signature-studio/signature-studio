// windows/settings-resize-a: the Settings window bottom edge is dragged down by 30 px: its height grows by it, its width stays.
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsResize } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-resize-a',
  covers: ['windows/op/resize'],
  async run({ guest }) {
    return settingsResize(guest, 30);
  },
});
