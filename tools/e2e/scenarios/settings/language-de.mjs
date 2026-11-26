// settings/language-de: settings/language/de (languageRun). item of the drop-down, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/language-de',
  covers: ["settings/language/de"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  run(ctx) {
    return S.languageRun(ctx, 1);
  },
});
