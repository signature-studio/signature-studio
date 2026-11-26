// settings/check-display-high-visibility: settings/check/display-high-visibility (checkRun). DPR 1; kept after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/check-display-high-visibility',
  covers: ["settings/check/display-high-visibility"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.checkRun(ctx, "General", /^Display/, 21, 316);
  },
});
