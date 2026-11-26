// settings/page-project-picture: settings/page/project (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-project-picture',
  covers: ["settings/page/project"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Project");
  },
});
