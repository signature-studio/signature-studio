// visual/theme-ppriime-dark-text: theme PPRIIME Dark: the menu bar's text is drawn (several colours, a luminance range).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeText } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ppriime-dark-text',
  covers: ['visual/text/topbar'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeText(ctx, "ppriime-dark"),
});
