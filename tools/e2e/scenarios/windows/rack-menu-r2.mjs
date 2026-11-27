// windows/rack-menu-r2: a right-click on channel 2's name opens one drop-down (TQuickPopupMenuWindow); Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import { rowMenu } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-menu-r2',
  covers: ['windows/rack/menu', 'windows/popup/right-click-channel-rack'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return rowMenu(guest, 1);
  },
});
