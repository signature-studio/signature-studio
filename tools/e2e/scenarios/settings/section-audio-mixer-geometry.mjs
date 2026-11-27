// settings/section-audio-mixer-geometry: settings/section/mixer (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-audio-mixer-geometry',
  covers: ["settings/section/mixer"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Audio", "Mixer", 'geometry');
  },
});
