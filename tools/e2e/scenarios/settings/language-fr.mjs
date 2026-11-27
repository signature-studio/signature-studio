// settings/language-fr: settings/language/fr (languageRun). item of the drop-down, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/language-fr',
  covers: ["settings/language/fr"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  expectStop: "new",
  run(ctx) {
    return S.languageRun(ctx, 3);
  },
});
