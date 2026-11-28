// settings/language-ja: settings/language/ja (languageRun). item of the drop-down, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/language-ja',
  covers: ["settings/language/ja"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  expectStop: "new",
  run(ctx) {
    return S.languageRun(ctx, 4);
  },
});
