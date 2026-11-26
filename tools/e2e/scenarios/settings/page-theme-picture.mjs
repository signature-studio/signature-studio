// settings/page-theme-picture: settings/page/theme (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-theme-picture',
  covers: ["settings/page/theme"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Theme");
  },
});
