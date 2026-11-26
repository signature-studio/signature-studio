// settings/section-theme-options-listed: settings/section/options (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-theme-options-listed',
  covers: ["settings/section/options"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Theme", "Options", 'listed');
  },
});
