// settings/theme-presets-grid-visible: settings/list/theme-tquickstringgrid-1 (controlCountRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/theme-presets-grid-visible',
  covers: ["settings/list/theme-tquickstringgrid-1"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.controlCountRun(ctx, 'Theme', 'TQuickStringGrid', 1);
  },
});
