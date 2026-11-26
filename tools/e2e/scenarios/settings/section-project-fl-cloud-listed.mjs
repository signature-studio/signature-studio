// settings/section-project-fl-cloud-listed: settings/section/fl-cloud (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-fl-cloud-listed',
  covers: ["settings/section/fl-cloud"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "FL Cloud", 'listed');
  },
});
