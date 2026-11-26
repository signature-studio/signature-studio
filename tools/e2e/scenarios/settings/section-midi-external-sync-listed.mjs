// settings/section-midi-external-sync-listed: settings/section/external-sync (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-midi-external-sync-listed',
  covers: ["settings/section/external-sync"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "MIDI", "External sync", 'listed');
  },
});
