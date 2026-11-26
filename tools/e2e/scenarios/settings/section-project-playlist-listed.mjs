// settings/section-project-playlist-listed: settings/section/playlist (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-project-playlist-listed',
  covers: ["settings/section/playlist"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "Project", "Playlist", 'listed');
  },
});
