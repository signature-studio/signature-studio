// windows/browser-tab-4: Browser tab 4 is pressed: the tab strip's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { browserTab } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/browser-tab-4',
  covers: ['windows/browser/tab-4'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return browserTab(guest, 3);
  },
});
