// settings/section-general-input-geometry: settings/section/input-2 (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-input-geometry',
  covers: ["settings/section/input-2"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Input", 'geometry');
  },
});
