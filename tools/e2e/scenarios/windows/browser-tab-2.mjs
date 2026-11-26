// windows/browser-tab-2: Browser tab 2 is pressed: the tab strip's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { browserTab } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/browser-tab-2',
  covers: ['windows/browser/tab-2'],
  async run({ guest }) {
    return browserTab(guest, 1);
  },
});
