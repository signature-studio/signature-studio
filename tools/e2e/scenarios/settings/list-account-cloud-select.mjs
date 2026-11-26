// settings/list-account-cloud-select: settings/list/account-tquickstringgrid-1 (controlCountRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/list-account-cloud-select',
  covers: ["settings/list/account-tquickstringgrid-1"],
  expectFail: null,
  run(ctx) {
    return S.controlCountRun(ctx, 'Account', 'TQuickStringGrid', 1);
  },
});
