// settings/language-vi: settings/language/vi (languageRun). item of the drop-down, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/language-vi',
  covers: ["settings/language/vi"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  run(ctx) {
    return S.languageRun(ctx, 6);
  },
});
