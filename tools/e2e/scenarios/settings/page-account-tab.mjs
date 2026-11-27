// settings/page-account-tab: settings/page/account, settings/selector/page-tabs (pageTabRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-account-tab',
  covers: ["settings/page/account","settings/selector/page-tabs"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pageTabRun(ctx, "Account");
  },
});
