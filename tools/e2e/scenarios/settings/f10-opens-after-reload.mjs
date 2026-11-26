// settings/f10-opens-after-reload: settings/window/f10 (f10ToggleRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/f10-opens-after-reload',
  covers: ["settings/window/f10"],
  expectFail: null,
  run(ctx) {
    return S.f10ToggleRun(ctx, { reload: true });
  },
});
