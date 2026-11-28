// settings/section-audio-input-output-geometry: settings/section/input-output (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-audio-input-output-geometry',
  covers: ["settings/section/input-output"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Audio", "Input / output", 'geometry');
  },
});
