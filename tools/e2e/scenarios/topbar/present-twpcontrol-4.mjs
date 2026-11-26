// topbar/present-twpcontrol-4: the fourth TWPControl is listed in the window table with no area (zero width): it is not drawn, so it has no place to operate on.
import { defineScenario } from '../../lib/scenario.mjs';
import { presentPart } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/present-twpcontrol-4',
  covers: ['topbar/part/twpcontrol-4'],
  async run({ guest }) {
    return presentPart(guest, 'twpcontrol-4');
  },
});
