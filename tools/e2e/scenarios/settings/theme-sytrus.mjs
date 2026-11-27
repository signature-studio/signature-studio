// settings/theme-sytrus: settings/theme/sytrus, settings/list/theme-tquickstringgrid-1 (themeRun). tile place of the Presets grid, DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-sytrus',
  covers: ["settings/theme/sytrus","settings/list/theme-tquickstringgrid-1"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: "new",
  run(ctx) {
    return S.themeRun(ctx, "Sytrus", 4, 0);
  },
});
