// settings/section-file-troubleshooting-geometry: settings/section/troubleshooting (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-troubleshooting-geometry',
  covers: ["settings/section/troubleshooting"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "Troubleshooting", 'geometry');
  },
});
