// settings/section-general-miscellaneous-listed: settings/section/miscellaneous (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-miscellaneous-listed',
  covers: ["settings/section/miscellaneous"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Miscellaneous", 'listed');
  },
});
