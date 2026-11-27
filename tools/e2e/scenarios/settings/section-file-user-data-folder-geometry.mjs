// settings/section-file-user-data-folder-geometry: settings/section/user-data-folder (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-user-data-folder-geometry',
  covers: ["settings/section/user-data-folder"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "User data folder", 'geometry');
  },
});
