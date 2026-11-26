// windows/settings-f11: F11 opens the Settings window on the project credits page (TMIDIForm, top-level).
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsOpen } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-f11',
  covers: ['windows/window/tmidiform-2'],
  async run({ guest }) {
    return settingsOpen(guest, 'F11');
  },
});
