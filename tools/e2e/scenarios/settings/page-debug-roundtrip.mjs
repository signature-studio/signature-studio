// settings/page-debug-roundtrip: settings/page/debug (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-debug-roundtrip',
  covers: ["settings/page/debug"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Debug");
  },
});
