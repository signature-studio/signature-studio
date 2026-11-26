// settings/section-general-system-listed: settings/section/system (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-system-listed',
  covers: ["settings/section/system"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "System", 'listed');
  },
});
