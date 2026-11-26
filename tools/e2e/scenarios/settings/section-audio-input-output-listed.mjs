// settings/section-audio-input-output-listed: settings/section/input-output (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-audio-input-output-listed',
  covers: ["settings/section/input-output"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Audio", "Input / output", 'listed');
  },
});
