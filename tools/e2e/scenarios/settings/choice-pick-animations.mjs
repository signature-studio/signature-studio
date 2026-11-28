// settings/choice-pick-animations: settings/choice/display-animations (choicePickRun). DPR 1; the colorful waveforms value is gated on the picture after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-pick-animations',
  covers: ["settings/choice/display-animations"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.choicePickRun(ctx, "General", /^Display/, 40, 205, 1);
  },
});
