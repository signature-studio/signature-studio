// topbar/wheel-tquickedittoolbar-2: wheel notches: the picture of the part changes, on the top bar's part tquickedittoolbar-2 (its place from the window table).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/wheel-tquickedittoolbar-2',
  covers: ['topbar/wheel/tquickedittoolbar-2', 'topbar/part/tquickedittoolbar-2'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'tquickedittoolbar-2', 'wheel');
  },
});
