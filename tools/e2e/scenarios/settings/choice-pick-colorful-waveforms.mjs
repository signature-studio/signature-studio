// settings/choice-pick-colorful-waveforms: settings/choice/display-colorful-waveforms (choicePickRun). DPR 1; the colorful waveforms value is gated on the picture after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-pick-colorful-waveforms',
  covers: ["settings/choice/display-colorful-waveforms"],
  displays: ["4k-dpr1"],
  expectFail: "new",
  run(ctx) {
    return S.choicePickRun(ctx, "General", /^Display/, 320, 320, 1, { gateKept: true });
  },
});
