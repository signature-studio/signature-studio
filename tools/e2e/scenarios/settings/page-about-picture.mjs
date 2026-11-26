// settings/page-about-picture: settings/page/about (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-about-picture',
  covers: ["settings/page/about"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "About");
  },
});
