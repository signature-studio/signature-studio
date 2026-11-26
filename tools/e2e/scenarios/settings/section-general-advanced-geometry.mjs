// settings/section-general-advanced-geometry: settings/section/advanced (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-advanced-geometry',
  covers: ["settings/section/advanced"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Advanced", 'geometry');
  },
});
