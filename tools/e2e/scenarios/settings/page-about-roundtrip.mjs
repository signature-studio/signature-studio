// settings/page-about-roundtrip: settings/page/about (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-about-roundtrip',
  covers: ["settings/page/about"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "About");
  },
});
