// topbar/present-tquickcontrolbar-2: the second TQuickControlBar is listed in the window table with no area (zero height): it is not drawn, so it has no place to operate on.
import { defineScenario } from '../../lib/scenario.mjs';
import { presentPart } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/present-tquickcontrolbar-2',
  covers: ['topbar/part/tquickcontrolbar-2'],
  async run({ guest }) {
    return presentPart(guest, 'tquickcontrolbar-2');
  },
});
