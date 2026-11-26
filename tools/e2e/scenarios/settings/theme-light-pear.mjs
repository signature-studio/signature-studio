// settings/theme-light-pear: settings/theme/light-pear, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-light-pear',
  covers: ["settings/theme/light-pear","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.themeRun(ctx, "Light pear", 3, 1);
  },
});
