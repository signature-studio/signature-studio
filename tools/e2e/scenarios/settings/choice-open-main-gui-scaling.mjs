// settings/choice-open-main-gui-scaling: settings/choice/display-main-gui-scaling (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-main-gui-scaling',
  covers: ["settings/choice/display-main-gui-scaling"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, "General", /^Display/, 40, 40);
  },
});
