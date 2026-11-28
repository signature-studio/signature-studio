// windows/plugin-esc: the plugin list is closed with Escape: it is gone from the table.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeByEscape } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/plugin-esc',
  covers: ['windows/plugin/close', 'windows/op/close'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return closeByEscape(guest, 'F8', 'TPlugListForm');
  },
});
