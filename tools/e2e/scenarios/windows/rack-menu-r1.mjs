// windows/rack-menu-r1: a right-click on channel 1's name opens one drop-down (TQuickPopupMenuWindow); Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import { rowMenu } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-menu-r1',
  covers: ['windows/rack/menu', 'windows/popup/right-click-channel-rack'],
  async run({ guest }) {
    return rowMenu(guest, 0);
  },
});
