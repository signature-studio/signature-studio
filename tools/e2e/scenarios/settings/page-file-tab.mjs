// settings/page-file-tab: settings/page/file, settings/selector/page-tabs (pageTabRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-file-tab',
  covers: ["settings/page/file","settings/selector/page-tabs"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.pageTabRun(ctx, "File");
  },
});
