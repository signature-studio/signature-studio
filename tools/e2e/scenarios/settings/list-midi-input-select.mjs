// settings/list-midi-input-select: settings/list/midi-tquickstringgrid-2, settings/value/midi-input-device (controlCountRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/list-midi-input-select',
  covers: ["settings/list/midi-tquickstringgrid-2","settings/value/midi-input-device"],
  expectFail: null,
  run(ctx) {
    return S.controlCountRun(ctx, 'MIDI', 'TQuickStringGrid', 2);
  },
});
