// settings/section-theme-options-geometry: settings/section/options (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-theme-options-geometry',
  covers: ["settings/section/options"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Theme", "Options", 'geometry');
  },
});
