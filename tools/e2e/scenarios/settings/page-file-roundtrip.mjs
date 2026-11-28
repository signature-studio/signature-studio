// settings/page-file-roundtrip: settings/page/file (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-file-roundtrip',
  covers: ["settings/page/file"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "File");
  },
});
