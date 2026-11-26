// settings/page-info-picture: settings/page/info (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-info-picture',
  covers: ["settings/page/info"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Info");
  },
});
