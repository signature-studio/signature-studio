// settings/info-fields-present: settings/control/info-tquickedit-1, settings/control/info-tquickedit-2, settings/control/info-tquickedit-3, settings/control/info-tquickedit-4 (controlCountRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/info-fields-present',
  covers: ["settings/control/info-tquickedit-1","settings/control/info-tquickedit-2","settings/control/info-tquickedit-3","settings/control/info-tquickedit-4"],
  expectFail: null,
  run(ctx) {
    return S.controlCountRun(ctx, 'Info', 'TQuickEdit', 4);
  },
});
