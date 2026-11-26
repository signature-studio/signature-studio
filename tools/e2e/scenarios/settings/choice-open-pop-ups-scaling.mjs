// settings/choice-open-pop-ups-scaling: settings/choice/display-pop-ups-scaling (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-pop-ups-scaling',
  covers: ["settings/choice/display-pop-ups-scaling"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, "General", /^Display/, 40, 124);
  },
});
