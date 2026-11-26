// settings/section-midi-output-geometry: settings/section/output (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-midi-output-geometry',
  covers: ["settings/section/output"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "MIDI", "Output", 'geometry');
  },
});
