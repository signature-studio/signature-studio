// windows/settings-f10: F10 opens the Settings window (TMIDIForm, top-level, the MIDI page).
import { defineScenario } from '../../lib/scenario.mjs';
import { settingsOpen } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-f10',
  covers: ['windows/window/tmidiform'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return settingsOpen(guest, 'F10');
  },
});
