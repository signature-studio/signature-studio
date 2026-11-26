// settings/section-general-system-geometry: settings/section/system (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-system-geometry',
  covers: ["settings/section/system"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "System", 'geometry');
  },
});
