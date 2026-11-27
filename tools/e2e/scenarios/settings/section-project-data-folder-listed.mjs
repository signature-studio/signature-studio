// settings/section-project-data-folder-listed: settings/section/data-folder (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-data-folder-listed',
  covers: ["settings/section/data-folder"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "Data folder", 'listed');
  },
});
