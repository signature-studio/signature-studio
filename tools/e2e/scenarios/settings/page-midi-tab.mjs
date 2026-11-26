// settings/page-midi-tab: settings/page/midi, settings/selector/page-tabs (pageTabRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-midi-tab',
  covers: ["settings/page/midi","settings/selector/page-tabs"],
  expectFail: null,
  run(ctx) {
    return S.pageTabRun(ctx, "MIDI");
  },
});
