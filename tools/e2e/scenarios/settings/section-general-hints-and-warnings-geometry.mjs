// settings/section-general-hints-and-warnings-geometry: settings/section/hints-and-warnings (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-hints-and-warnings-geometry',
  covers: ["settings/section/hints-and-warnings"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Hints and warnings", 'geometry');
  },
});
