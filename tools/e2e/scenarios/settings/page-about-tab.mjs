// settings/page-about-tab: settings/page/about, settings/selector/page-tabs (pageTabRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-about-tab',
  covers: ["settings/page/about","settings/selector/page-tabs"],
  expectFail: null,
  run(ctx) {
    return S.pageTabRun(ctx, "About");
  },
});
