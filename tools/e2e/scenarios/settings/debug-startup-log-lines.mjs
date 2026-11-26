// settings/debug-startup-log-lines: settings/page/debug, settings/control/debug-tquickmemo-1, settings/value/debug-log (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/debug-startup-log-lines',
  covers: ["settings/page/debug","settings/control/debug-tquickmemo-1","settings/value/debug-log"],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'Debug', ['Browser caching took', 'There were no updates for'], 50, /Startup/);
  },
});
