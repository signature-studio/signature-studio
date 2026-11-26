// visual/theme-ppriime-light-text: theme PPRIIME Light: the menu bar's text is drawn (several colours, a luminance range).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeText } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ppriime-light-text',
  covers: ['visual/text/topbar'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeText(ctx, "ppriime-light"),
});
