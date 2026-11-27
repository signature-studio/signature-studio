// settings/page-debug-picture: settings/page/debug (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-debug-picture',
  covers: ["settings/page/debug"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Debug");
  },
});
