// settings/section-general-miscellaneous-listed: settings/section/miscellaneous (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-miscellaneous-listed',
  covers: ["settings/section/miscellaneous"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Miscellaneous", 'listed');
  },
});
