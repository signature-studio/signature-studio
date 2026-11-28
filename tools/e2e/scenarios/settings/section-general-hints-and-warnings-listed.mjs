// settings/section-general-hints-and-warnings-listed: settings/section/hints-and-warnings (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-hints-and-warnings-listed',
  covers: ["settings/section/hints-and-warnings"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Hints and warnings", 'listed');
  },
});
