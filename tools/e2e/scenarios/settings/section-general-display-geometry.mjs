// settings/section-general-display-geometry: settings/section/display-requires-a-restart-to-take-effect (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-display-geometry',
  covers: ["settings/section/display-requires-a-restart-to-take-effect"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", /^Display/, 'geometry');
  },
});
