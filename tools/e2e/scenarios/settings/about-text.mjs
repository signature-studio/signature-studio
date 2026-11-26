// settings/about-text: settings/page/about, settings/control/about-tquickmemo-1, settings/value/about-text (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/about-text',
  covers: ["settings/page/about","settings/control/about-tquickmemo-1","settings/value/about-text"],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'About', ['FL Studio'], 1, null);
  },
});
