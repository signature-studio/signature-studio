// settings/section-account-fl-studio-license-listed: settings/section/fl-studio-license (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-account-fl-studio-license-listed',
  covers: ["settings/section/fl-studio-license"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Account", /^FL Studio license/, 'listed');
  },
});
