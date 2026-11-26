// windows/settings-resize-b: the Settings window bottom edge is dragged down by 60 px: its height grows by it, its width stays.
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsResize } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-resize-b',
  covers: ['windows/op/resize'],
  async run({ guest }) {
    return settingsResize(guest, 60);
  },
});
