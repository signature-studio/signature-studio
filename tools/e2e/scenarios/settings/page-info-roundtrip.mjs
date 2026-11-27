// settings/page-info-roundtrip: settings/page/info (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-info-roundtrip',
  covers: ["settings/page/info"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Info");
  },
});
