// visual/language-vi-settings: language vi: the Settings window's picture is the golden of the language (language.tsv). The language is applied after a restart (#1395 is open): this is the screen after the choice, so it does not yet show the language.
import { defineScenario } from '../../lib/scenario.mjs';
import { languageSettings } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/language-vi-settings',
  covers: ['visual/language/vi'],
  displays: ['4k-dpr1'],
  expectFail: '#1395',
  run: (ctx) => languageSettings(ctx, "vi"),
});
