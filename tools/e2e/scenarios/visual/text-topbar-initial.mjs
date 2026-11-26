// visual/text-topbar-initial: at the start, the menu bar's text is drawn (several colours, a luminance range).
import { defineScenario } from '../../lib/scenario.mjs';
import { textInitial } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/text-topbar-initial',
  covers: ['visual/text/topbar'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => textInitial(ctx),
});
