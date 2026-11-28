// windows/settings-reopen: the Settings window is closed with Escape and opened again with F10.
import { defineScenario } from '../../lib/scenario.mjs';
import { reopenSettings } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-reopen',
  covers: ['windows/op/close', 'windows/window/tmidiform'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return reopenSettings(guest);
  },
});
