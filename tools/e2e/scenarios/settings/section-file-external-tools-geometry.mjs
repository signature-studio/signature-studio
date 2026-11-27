// settings/section-file-external-tools-geometry: settings/section/external-tools (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-external-tools-geometry',
  covers: ["settings/section/external-tools"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "External tools", 'geometry');
  },
});
