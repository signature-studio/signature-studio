// settings/section-file-external-tools-geometry: settings/section/external-tools (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-external-tools-geometry',
  covers: ["settings/section/external-tools"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "External tools", 'geometry');
  },
});
