// settings/section-project-time-settings-geometry: settings/section/time-settings (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-time-settings-geometry',
  covers: ["settings/section/time-settings"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "Time settings", 'geometry');
  },
});
