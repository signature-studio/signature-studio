// windows/settings-move-a: the Settings window is dragged by its title by 120 x 60 guest px: its rectangle moves by that.
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsMove } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-move-a',
  covers: ['windows/op/move', 'windows/window/tmidiform'],
  async run({ guest }) {
    return settingsMove(guest, 120, 60);
  },
});
