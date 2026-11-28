// settings/choice-open-toolbars-scaling: settings/choice/display-toolbars-scaling (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-toolbars-scaling',
  covers: ["settings/choice/display-toolbars-scaling"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, "General", /^Display/, 40, 151);
  },
});
