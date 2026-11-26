// settings/section-general-language-listed: settings/section/language-requires-a-restart-to-take-effect (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-general-language-listed',
  covers: ["settings/section/language-requires-a-restart-to-take-effect"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "General", /^Language/, 'listed');
  },
});
