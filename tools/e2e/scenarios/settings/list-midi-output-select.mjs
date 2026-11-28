// settings/list-midi-output-select: settings/list/midi-tquickstringgrid-1, settings/value/midi-output-device (gridRun). offsets at DPR 1.
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/list-midi-output-select',
  covers: ["settings/list/midi-tquickstringgrid-1","settings/value/midi-output-device"],
  displays: ["4k-dpr1", "4k-dpr2"],
  expectFail: null,
  run(ctx) {
    return S.gridRun(ctx, "MIDI", "Output", 0);
  },
});
