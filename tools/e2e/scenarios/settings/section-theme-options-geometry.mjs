// settings/section-theme-options-geometry: settings/section/options (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-theme-options-geometry',
  covers: ["settings/section/options"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Theme", "Options", 'geometry');
  },
});
