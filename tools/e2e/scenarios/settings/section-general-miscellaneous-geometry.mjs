// settings/section-general-miscellaneous-geometry: settings/section/miscellaneous (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-miscellaneous-geometry',
  covers: ["settings/section/miscellaneous"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Miscellaneous", 'geometry');
  },
});
