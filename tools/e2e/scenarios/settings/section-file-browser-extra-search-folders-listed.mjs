// settings/section-file-browser-extra-search-folders-listed: settings/section/browser-extra-search-folders (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-browser-extra-search-folders-listed',
  covers: ["settings/section/browser-extra-search-folders"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "Browser extra search folders", 'listed');
  },
});
