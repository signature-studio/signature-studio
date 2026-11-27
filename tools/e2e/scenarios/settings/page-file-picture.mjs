// settings/page-file-picture: settings/page/file (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-file-picture',
  covers: ["settings/page/file"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "File");
  },
});
