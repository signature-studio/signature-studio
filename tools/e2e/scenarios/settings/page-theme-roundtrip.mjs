// settings/page-theme-roundtrip: settings/page/theme (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-theme-roundtrip',
  covers: ["settings/page/theme"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Theme");
  },
});
