// settings/choice-open-audio-output: settings/choice/audio-output-device (choiceOpenRun). DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-open-audio-output',
  covers: ["settings/choice/audio-output-device"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.choiceOpenRun(ctx, "Audio", /^Input \/ output/, 100, 40);
  },
});
