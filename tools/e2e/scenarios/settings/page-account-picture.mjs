// settings/page-account-picture: settings/page/account (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-account-picture',
  covers: ["settings/page/account"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Account");
  },
});
