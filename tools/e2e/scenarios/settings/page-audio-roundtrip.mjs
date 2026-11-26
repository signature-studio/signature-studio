// settings/page-audio-roundtrip: settings/page/audio (pageRoundtripRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-audio-roundtrip',
  covers: ["settings/page/audio"],
  expectFail: null,
  run(ctx) {
    return S.pageRoundtripRun(ctx, "Audio");
  },
});
