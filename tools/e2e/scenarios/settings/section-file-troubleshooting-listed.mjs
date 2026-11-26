// settings/section-file-troubleshooting-listed: settings/section/troubleshooting (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-troubleshooting-listed',
  covers: ["settings/section/troubleshooting"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "Troubleshooting", 'listed');
  },
});
