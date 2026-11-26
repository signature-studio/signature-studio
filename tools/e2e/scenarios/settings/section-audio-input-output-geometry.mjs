// settings/section-audio-input-output-geometry: settings/section/input-output (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-audio-input-output-geometry',
  covers: ["settings/section/input-output"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Audio", "Input / output", 'geometry');
  },
});
