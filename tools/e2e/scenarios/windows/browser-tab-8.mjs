// windows/browser-tab-8: Browser tab 8 is pressed: the tab strip's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { browserTab } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/browser-tab-8',
  covers: ['windows/browser/tab-8'],
  async run({ guest }) {
    return browserTab(guest, 7);
  },
});
