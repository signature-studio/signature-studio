// settings/page-theme-picture: settings/page/theme (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-theme-picture',
  covers: ["settings/page/theme"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Theme");
  },
});
