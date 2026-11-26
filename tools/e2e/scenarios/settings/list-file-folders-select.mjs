// settings/list-file-folders-select: settings/list/file-tquickstringgrid-1 (gridRun). offsets at DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/list-file-folders-select',
  covers: ["settings/list/file-tquickstringgrid-1"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.gridRun(ctx, "File", "Browser extra search folders", 0);
  },
});
