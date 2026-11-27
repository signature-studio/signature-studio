// settings/choice-open-animations: settings/choice/display-animations (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-animations',
  covers: ["settings/choice/display-animations"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, "General", /^Display/, 40, 205);
  },
});
