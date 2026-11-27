// settings/choice-language-opens: settings/choice/general-language (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-language-opens',
  covers: ["settings/choice/general-language"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, 'General', /^Language/, 60, 30);
  },
});
