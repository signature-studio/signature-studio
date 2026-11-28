// settings/about-text: settings/page/about, settings/control/about-tquickmemo-1, settings/value/about-text (memoryTextRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/about-text',
  covers: ["settings/page/about","settings/control/about-tquickmemo-1","settings/value/about-text"],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  expectFail: null,
  run(ctx) {
    return S.memoryTextRun(ctx, 'About', ['FL Studio'], 1, null);
  },
});
