// windows/settings-esc: the Settings window is closed with Escape: it is gone from the table.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeByEscape } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/settings-esc',
  covers: ['windows/op/close'],
  async run({ guest }) {
    return closeByEscape(guest, 'F10', 'TMIDIForm');
  },
});
