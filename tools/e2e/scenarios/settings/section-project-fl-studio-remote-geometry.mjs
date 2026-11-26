// settings/section-project-fl-studio-remote-geometry: settings/section/fl-studio-remote (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-fl-studio-remote-geometry',
  covers: ["settings/section/fl-studio-remote"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "FL Studio Remote", 'geometry');
  },
});
