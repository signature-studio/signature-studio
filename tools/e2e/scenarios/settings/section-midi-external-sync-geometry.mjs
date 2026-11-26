// settings/section-midi-external-sync-geometry: settings/section/external-sync (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-midi-external-sync-geometry',
  covers: ["settings/section/external-sync"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "MIDI", "External sync", 'geometry');
  },
});
