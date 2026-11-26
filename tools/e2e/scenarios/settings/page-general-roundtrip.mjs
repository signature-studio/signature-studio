// settings/page-general-roundtrip: settings/page/general (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-general-roundtrip',
  covers: ["settings/page/general"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "General");
  },
});
