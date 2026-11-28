// settings/section-general-advanced-listed: settings/section/advanced (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-advanced-listed',
  covers: ["settings/section/advanced"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Advanced", 'listed');
  },
});
