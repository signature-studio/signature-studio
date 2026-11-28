// settings/section-project-advanced-listed: settings/section/advanced-2 (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-advanced-listed',
  covers: ["settings/section/advanced-2"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "Advanced", 'listed');
  },
});
