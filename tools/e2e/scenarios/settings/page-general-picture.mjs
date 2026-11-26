// settings/page-general-picture: settings/page/general (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-general-picture',
  covers: ["settings/page/general"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "General");
  },
});
