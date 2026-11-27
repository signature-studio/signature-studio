// settings/section-project-fl-cloud-listed: settings/section/fl-cloud (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-fl-cloud-listed',
  covers: ["settings/section/fl-cloud"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "FL Cloud", 'listed');
  },
});
