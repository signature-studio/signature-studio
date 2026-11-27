// settings/project-data-folder-path: settings/section/data-folder, settings/control/project-tquickedit-1, settings/value/data-folder-path (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/project-data-folder-path',
  covers: ["settings/section/data-folder","settings/control/project-tquickedit-1","settings/value/data-folder-path"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'Project', ['Image-Line\\FL Studio'], 1, null);
  },
});
