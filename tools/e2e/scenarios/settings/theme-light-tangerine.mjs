// settings/theme-light-tangerine: settings/theme/light-tangerine, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-light-tangerine',
  covers: ["settings/theme/light-tangerine","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.themeRun(ctx, "Light tangerine", 3, 2);
  },
});
