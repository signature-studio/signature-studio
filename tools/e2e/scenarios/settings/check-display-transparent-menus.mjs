// settings/check-display-transparent-menus: settings/check/display-transparent-menus (checkRun). DPR 1; kept after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/check-display-transparent-menus',
  covers: ["settings/check/display-transparent-menus"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.checkRun(ctx, "General", /^Display/, 309, 287);
  },
});
