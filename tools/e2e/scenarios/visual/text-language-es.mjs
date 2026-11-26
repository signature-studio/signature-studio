// visual/text-language-es: language es: the menu bar's text is drawn (several colours, a luminance range).
import { defineScenario } from '../../lib/scenario.mjs';
import { textLanguage } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/text-language-es',
  covers: ['visual/text/topbar'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => textLanguage(ctx, "es"),
});
