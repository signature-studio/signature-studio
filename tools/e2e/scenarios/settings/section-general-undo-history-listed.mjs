// settings/section-general-undo-history-listed: settings/section/undo-history (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-undo-history-listed',
  covers: ["settings/section/undo-history"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Undo history", 'listed');
  },
});
