// settings/theme-dedzii: settings/theme/dedzii, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-dedzii',
  covers: ["settings/theme/dedzii","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.themeRun(ctx, "DEDZII", 0, 2);
  },
});
