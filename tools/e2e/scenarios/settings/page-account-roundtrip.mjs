// settings/page-account-roundtrip: settings/page/account (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-account-roundtrip',
  covers: ["settings/page/account"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Account");
  },
});
