// settings/choice-pick-pop-ups-scaling: settings/choice/display-pop-ups-scaling (choicePickRun). DPR 1; the colorful waveforms value is gated on the picture after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-pick-pop-ups-scaling',
  covers: ["settings/choice/display-pop-ups-scaling"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.choicePickRun(ctx, "General", /^Display/, 40, 124, 1);
  },
});
