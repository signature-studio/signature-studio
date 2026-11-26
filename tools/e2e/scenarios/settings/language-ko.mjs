// settings/language-ko: settings/language/ko (languageRun). item of the drop-down, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/language-ko',
  covers: ["settings/language/ko"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  run(ctx) {
    return S.languageRun(ctx, 5);
  },
});
