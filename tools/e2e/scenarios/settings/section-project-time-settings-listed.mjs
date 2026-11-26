// settings/section-project-time-settings-listed: settings/section/time-settings (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-time-settings-listed',
  covers: ["settings/section/time-settings"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "Time settings", 'listed');
  },
});
