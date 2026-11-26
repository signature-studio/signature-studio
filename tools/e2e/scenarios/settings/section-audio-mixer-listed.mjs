// settings/section-audio-mixer-listed: settings/section/mixer (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-audio-mixer-listed',
  covers: ["settings/section/mixer"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Audio", "Mixer", 'listed');
  },
});
