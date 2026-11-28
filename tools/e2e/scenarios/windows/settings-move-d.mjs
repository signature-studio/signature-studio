// windows/settings-move-d: the Settings window is dragged by 40 x -40 guest px: its rectangle moves by that.
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsMove } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-move-d',
  covers: ['windows/op/move'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return settingsMove(guest, 40, -40);
  },
});
