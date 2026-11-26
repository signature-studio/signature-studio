// settings/section-account-fl-studio-license-geometry: settings/section/fl-studio-license (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-account-fl-studio-license-geometry',
  covers: ["settings/section/fl-studio-license"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Account", /^FL Studio license/, 'geometry');
  },
});
