// settings/section-project-advanced-geometry: settings/section/advanced-2 (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-advanced-geometry',
  covers: ["settings/section/advanced-2"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "Advanced", 'geometry');
  },
});
