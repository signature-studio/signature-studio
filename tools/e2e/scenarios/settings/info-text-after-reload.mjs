// settings/info-text-after-reload: settings/control/info-tquickedit-1 (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/info-text-after-reload',
  covers: ["settings/control/info-tquickedit-1"],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'Info', ['Image-Line forums'], 1, null, { reload: true });
  },
});
