// settings/info-text-visualizer: settings/page/info, settings/control/info-tquickmemo-1, settings/value/info-text (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/info-text-visualizer',
  covers: ["settings/page/info","settings/control/info-tquickmemo-1","settings/value/info-text"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'Info', ['Image-Line forums'], 1, null);
  },
});
