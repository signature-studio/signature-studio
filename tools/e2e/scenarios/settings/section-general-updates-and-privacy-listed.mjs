// settings/section-general-updates-and-privacy-listed: settings/section/updates-and-privacy (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-updates-and-privacy-listed',
  covers: ["settings/section/updates-and-privacy"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", "Updates and privacy", 'listed');
  },
});
