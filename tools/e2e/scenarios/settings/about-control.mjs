// settings/about-control: settings/control/about-twpcontrol-1 (controlCountRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/about-control',
  covers: ["settings/control/about-twpcontrol-1"],
  expectFail: null,
  run(ctx) {
    return S.controlCountRun(ctx, 'About', 'TWPControl', 1);
  },
});
