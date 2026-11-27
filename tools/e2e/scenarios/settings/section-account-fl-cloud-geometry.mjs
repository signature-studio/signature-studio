// settings/section-account-fl-cloud-geometry: settings/section/fl-cloud-2 (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-account-fl-cloud-geometry',
  covers: ["settings/section/fl-cloud-2"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Account", "FL Cloud", 'geometry');
  },
});
