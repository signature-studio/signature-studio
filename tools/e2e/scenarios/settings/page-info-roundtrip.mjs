// settings/page-info-roundtrip: settings/page/info (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-info-roundtrip',
  covers: ["settings/page/info"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Info");
  },
});
