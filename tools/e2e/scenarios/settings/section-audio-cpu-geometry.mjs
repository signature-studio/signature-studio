// settings/section-audio-cpu-geometry: settings/section/cpu (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-audio-cpu-geometry',
  covers: ["settings/section/cpu"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Audio", "CPU", 'geometry');
  },
});
