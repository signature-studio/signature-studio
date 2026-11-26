// settings/choice-open-colorful-waveforms: settings/choice/display-colorful-waveforms (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-colorful-waveforms',
  covers: ["settings/choice/display-colorful-waveforms"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, "General", /^Display/, 320, 320);
  },
});
