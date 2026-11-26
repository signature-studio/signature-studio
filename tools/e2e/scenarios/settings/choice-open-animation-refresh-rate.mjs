// settings/choice-open-animation-refresh-rate: settings/choice/display-animation-refresh-rate (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-animation-refresh-rate',
  covers: ["settings/choice/display-animation-refresh-rate"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, "General", /^Display/, 40, 266);
  },
});
