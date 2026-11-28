// settings/f10-toggles-the-window: settings/window/f10 (f10ToggleRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/f10-toggles-the-window',
  covers: ["settings/window/f10"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.f10ToggleRun(ctx);
  },
});
