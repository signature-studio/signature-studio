// settings/section-general-undo-history-geometry: settings/section/undo-history (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-undo-history-geometry',
  covers: ["settings/section/undo-history"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Undo history", 'geometry');
  },
});
