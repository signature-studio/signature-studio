// settings/page-audio-picture: settings/page/audio (pagePictureRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/page-audio-picture',
  covers: ["settings/page/audio"],
  expectFail: null,
  run(ctx) {
    return S.pagePictureRun(ctx, "Audio");
  },
});
