// windows/browser-tab-5: Browser tab 5 is pressed: the tab strip's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { browserTab } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/browser-tab-5',
  covers: ['windows/browser/tab-5'],
  async run({ guest }) {
    return browserTab(guest, 4);
  },
});
