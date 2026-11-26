// visual/language-ja-black: language ja: no all-black 64 x 64 tile is drawn after the language is set.
import { defineScenario } from '../../lib/scenario.mjs';
import { languageBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/language-ja-black',
  covers: ['visual/black/after-language'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => languageBlack(ctx, "ja"),
});
