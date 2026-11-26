// windows/rack-plus: the + under the channels: the rack changes or a drop-down opens.
import { defineScenario } from '../../lib/scenario.mjs';
import { plusButton } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-plus',
  covers: ['windows/rack/add'],
  async run({ guest }) {
    return plusButton(guest);
  },
});
