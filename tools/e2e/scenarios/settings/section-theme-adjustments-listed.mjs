// settings/section-theme-adjustments-listed: settings/section/adjustments (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-theme-adjustments-listed',
  covers: ["settings/section/adjustments"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Theme", "Adjustments", 'listed');
  },
});
