// settings/theme-dark: settings/theme/dark, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-dark',
  covers: ["settings/theme/dark","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.themeRun(ctx, "Dark", 2, 2);
  },
});
