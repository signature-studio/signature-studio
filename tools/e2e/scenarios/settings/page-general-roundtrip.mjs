// settings/page-general-roundtrip: settings/page/general (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-general-roundtrip',
  covers: ["settings/page/general"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "General");
  },
});
