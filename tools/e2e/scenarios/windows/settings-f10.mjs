// windows/settings-f10: F10 opens the Settings window (TMIDIForm, top-level, the MIDI page).
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsOpen } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-f10',
  covers: ['windows/window/tmidiform'],
  async run({ guest }) {
    return settingsOpen(guest, 'F10');
  },
});
