// settings/file-user-data-folder-path: settings/section/user-data-folder (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/file-user-data-folder-path',
  covers: ["settings/section/user-data-folder"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'File', ['Documents\\Image-Line'], 1, null);
  },
});
