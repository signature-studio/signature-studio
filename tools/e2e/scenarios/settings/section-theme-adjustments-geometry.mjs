// settings/section-theme-adjustments-geometry: settings/section/adjustments (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-theme-adjustments-geometry',
  covers: ["settings/section/adjustments"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Theme", "Adjustments", 'geometry');
  },
});
