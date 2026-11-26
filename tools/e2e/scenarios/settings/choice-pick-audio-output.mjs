// settings/choice-pick-audio-output: settings/choice/audio-output-device (choicePickRun). DPR 1; the colorful waveforms value is gated on the picture after a reload.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/choice-pick-audio-output',
  covers: ["settings/choice/audio-output-device"],
  displays: ["4k-dpr1"],
  expectFail: null,
  run(ctx) {
    return S.choicePickRun(ctx, "Audio", /^Input \/ output/, 100, 40, 1);
  },
});
