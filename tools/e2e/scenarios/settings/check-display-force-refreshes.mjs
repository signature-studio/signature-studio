// settings/check-display-force-refreshes: settings/check/display-force-refreshes (checkRun). DPR 1; kept after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/check-display-force-refreshes',
  covers: ["settings/check/display-force-refreshes"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: "new",
  run(ctx) {
    return S.checkRun(ctx, "General", /^Display/, 309, 266);
  },
});
