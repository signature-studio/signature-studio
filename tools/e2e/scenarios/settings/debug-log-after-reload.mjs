// settings/debug-log-after-reload: settings/control/debug-tquickmemo-1 (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/debug-log-after-reload',
  covers: ["settings/control/debug-tquickmemo-1"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'Debug', ['Browser caching took', 'There were no updates for'], 50, /Startup/, { reload: true });
  },
});
