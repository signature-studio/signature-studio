// windows/settings-move-b: the Settings window is dragged back by -100 x -50 guest px: its rectangle moves by that.
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsMove } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-move-b',
  covers: ['windows/op/move'],
  async run({ guest }) {
    return settingsMove(guest, -100, -50);
  },
});
