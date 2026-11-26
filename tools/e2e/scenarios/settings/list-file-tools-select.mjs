// settings/list-file-tools-select: settings/list/file-tquickstringgrid-2 (gridRun). offsets at DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/list-file-tools-select',
  covers: ["settings/list/file-tquickstringgrid-2"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.gridRun(ctx, "File", "External tools", 0);
  },
});
