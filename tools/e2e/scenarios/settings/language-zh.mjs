// settings/language-zh: settings/language/zh (languageRun). item of the drop-down, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/language-zh',
  covers: ["settings/language/zh"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  expectStop: "new",
  run(ctx) {
    return S.languageRun(ctx, 7);
  },
});
