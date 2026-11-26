// settings/section-audio-cpu-listed: settings/section/cpu (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-audio-cpu-listed',
  covers: ["settings/section/cpu"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Audio", "CPU", 'listed');
  },
});
