// settings/section-midi-input-listed: settings/section/input (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-midi-input-listed',
  covers: ["settings/section/input"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "MIDI", "Input", 'listed');
  },
});
