// settings/page-file-picture: settings/page/file (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-file-picture',
  covers: ["settings/page/file"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "File");
  },
});
