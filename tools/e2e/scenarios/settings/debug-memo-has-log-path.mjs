// settings/debug-memo-has-log-path: settings/value/debug-log (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/debug-memo-has-log-path',
  covers: ["settings/value/debug-log"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'Debug', ['FL Studio\\Support\\Logs'], 1, null);
  },
});
