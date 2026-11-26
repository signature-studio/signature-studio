// settings/theme-ppriime-dark: settings/theme/ppriime-dark, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-ppriime-dark',
  covers: ["settings/theme/ppriime-dark","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.themeRun(ctx, "PPRIIME Dark", 1, 0);
  },
});
