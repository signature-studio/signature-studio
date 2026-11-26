// settings/theme-light-cherry: settings/theme/light-cherry, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-light-cherry',
  covers: ["settings/theme/light-cherry","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.themeRun(ctx, "Light cherry", 3, 0);
  },
});
