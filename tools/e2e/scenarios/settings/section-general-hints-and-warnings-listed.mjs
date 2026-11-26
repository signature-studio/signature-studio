// settings/section-general-hints-and-warnings-listed: settings/section/hints-and-warnings (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-hints-and-warnings-listed',
  covers: ["settings/section/hints-and-warnings"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Hints and warnings", 'listed');
  },
});
