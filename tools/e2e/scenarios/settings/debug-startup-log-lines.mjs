// settings/debug-startup-log-lines: settings/page/debug, settings/control/debug-tquickmemo-1, settings/value/debug-log (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/debug-startup-log-lines',
  covers: ["settings/page/debug","settings/control/debug-tquickmemo-1","settings/value/debug-log"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'Debug', ['Browser caching took', 'There were no updates for'], 50, /Startup/);
  },
});
