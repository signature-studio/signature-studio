// settings/language-es: settings/language/es (languageRun). item of the drop-down, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/language-es',
  covers: ["settings/language/es"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  expectStop: "new",
  run(ctx) {
    return S.languageRun(ctx, 2);
  },
});
