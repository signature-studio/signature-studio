// settings/page-midi-roundtrip: settings/page/midi (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-midi-roundtrip',
  covers: ["settings/page/midi"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "MIDI");
  },
});
