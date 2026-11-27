// windows/settings-resize-c: the Settings window bottom edge is dragged up by 30 px: its height shrinks by it.
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsResize } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-resize-c',
  covers: ['windows/op/resize'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return settingsResize(guest, -30);
  },
});
