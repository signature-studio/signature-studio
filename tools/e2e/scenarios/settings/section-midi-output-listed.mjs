// settings/section-midi-output-listed: settings/section/output (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-midi-output-listed',
  covers: ["settings/section/output"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "MIDI", "Output", 'listed');
  },
});
