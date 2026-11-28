// settings/section-theme-presets-geometry: settings/section/presets (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-theme-presets-geometry',
  covers: ["settings/section/presets"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Theme", "Presets", 'geometry');
  },
});
