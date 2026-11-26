// visual/theme-dedzii-light-text: theme DEDZII Light: the menu bar's text is drawn (several colours, a luminance range).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeText } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-dedzii-light-text',
  covers: ['visual/text/topbar'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeText(ctx, "dedzii-light"),
});
