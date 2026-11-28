// settings/section-general-updates-and-privacy-geometry: settings/section/updates-and-privacy (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-updates-and-privacy-geometry',
  covers: ["settings/section/updates-and-privacy"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Updates and privacy", 'geometry');
  },
});
