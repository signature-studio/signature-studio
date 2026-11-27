// settings/page-debug-roundtrip: settings/page/debug (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-debug-roundtrip',
  covers: ["settings/page/debug"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Debug");
  },
});
