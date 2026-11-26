// windows/browser-tab-4: Browser tab 4 is pressed: the tab strip's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { browserTab } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/browser-tab-4',
  covers: ['windows/browser/tab-4'],
  async run({ guest }) {
    return browserTab(guest, 3);
  },
});
