// settings/about-control: settings/control/about-twpcontrol-1 (controlCountRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/about-control',
  covers: ["settings/control/about-twpcontrol-1"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.controlCountRun(ctx, 'About', 'TWPControl', 1);
  },
});
