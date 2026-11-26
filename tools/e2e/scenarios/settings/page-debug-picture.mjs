// settings/page-debug-picture: settings/page/debug (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-debug-picture',
  covers: ["settings/page/debug"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Debug");
  },
});
