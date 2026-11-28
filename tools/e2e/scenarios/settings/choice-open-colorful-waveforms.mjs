// settings/choice-open-colorful-waveforms: settings/choice/display-colorful-waveforms (choiceOpenRun). The drop-down of Colorful waveforms does not open after File > New at DPR 1 and DPR 2 (a black box covers it; the window table has no window there: the other drop-downs of the page open). Known defect (new). The place is the drop-down's centre (DPR 1: 375, 311).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-colorful-waveforms',
  covers: ["settings/choice/display-colorful-waveforms"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: "new",
  run(ctx) {
    return S.choiceOpenRun(ctx, "General", /^Display/, 375, 311);
  },
});
