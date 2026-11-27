// windows/settings-move-c: the Settings window is dragged down by 80 guest px: its rectangle moves by that.
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsMove } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-move-c',
  covers: ['windows/op/move'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return settingsMove(guest, 0, 80);
  },
});
