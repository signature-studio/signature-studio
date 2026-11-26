// settings/page-account-tab: settings/page/account, settings/selector/page-tabs (pageTabRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-account-tab',
  covers: ["settings/page/account","settings/selector/page-tabs"],
  expectFail: null,
  run(ctx) {
    return S.pageTabRun(ctx, "Account");
  },
});
