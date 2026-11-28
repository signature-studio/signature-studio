// topbar/click-splitter-1: a left click alone does not move the splitter (the picture of its place and the windows stay as they are); the splitter moves by a drag (drag-splitter-1).
import { defineScenario } from '../../lib/scenario.mjs';
import { partOperation } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/click-splitter-1',
  covers: ['topbar/click/splitter-1', 'topbar/part/splitter-1'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return partOperation(guest, 'splitter-1', 'click');
  },
});
