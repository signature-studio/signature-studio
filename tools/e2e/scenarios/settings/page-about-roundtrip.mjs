// settings/page-about-roundtrip: settings/page/about (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-about-roundtrip',
  covers: ["settings/page/about"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "About");
  },
});
