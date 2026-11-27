// settings/check-input-click-and-hold: settings/check/input-click-and-hold (checkRun). DPR 1; kept after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/check-input-click-and-hold',
  covers: ["settings/check/input-click-and-hold"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.checkRun(ctx, "General", "Input", 21, 65);
  },
});
