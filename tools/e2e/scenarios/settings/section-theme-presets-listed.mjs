// settings/section-theme-presets-listed: settings/section/presets (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-theme-presets-listed',
  covers: ["settings/section/presets"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Theme", "Presets", 'listed');
  },
});
