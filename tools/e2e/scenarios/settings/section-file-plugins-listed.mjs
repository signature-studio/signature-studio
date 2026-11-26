// settings/section-file-plugins-listed: settings/section/plugins (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-plugins-listed',
  covers: ["settings/section/plugins"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "Plugins", 'listed');
  },
});
