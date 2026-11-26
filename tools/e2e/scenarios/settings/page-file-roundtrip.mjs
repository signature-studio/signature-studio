// settings/page-file-roundtrip: settings/page/file (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-file-roundtrip',
  covers: ["settings/page/file"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "File");
  },
});
