// settings/section-file-user-data-folder-listed: settings/section/user-data-folder (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-user-data-folder-listed',
  covers: ["settings/section/user-data-folder"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "User data folder", 'listed');
  },
});
