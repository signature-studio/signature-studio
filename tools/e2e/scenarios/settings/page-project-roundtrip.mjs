// settings/page-project-roundtrip: settings/page/project (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-project-roundtrip',
  covers: ["settings/page/project"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Project");
  },
});
