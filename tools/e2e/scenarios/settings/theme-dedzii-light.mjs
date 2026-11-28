// settings/theme-dedzii-light: settings/theme/dedzii-light, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-dedzii-light',
  covers: ["settings/theme/dedzii-light","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.themeRun(ctx, "DEDZII Light", 0, 1);
  },
});
