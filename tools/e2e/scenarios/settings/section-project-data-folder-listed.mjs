// settings/section-project-data-folder-listed: settings/section/data-folder (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-data-folder-listed',
  covers: ["settings/section/data-folder"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "Data folder", 'listed');
  },
});
