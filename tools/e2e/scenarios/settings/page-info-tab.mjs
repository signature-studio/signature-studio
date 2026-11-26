// settings/page-info-tab: settings/page/info, settings/selector/page-tabs (pageTabRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-info-tab',
  covers: ["settings/page/info","settings/selector/page-tabs"],
  expectFail: null,
  run(ctx) {
    return S.pageTabRun(ctx, "Info");
  },
});
