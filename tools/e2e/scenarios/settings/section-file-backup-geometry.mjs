// settings/section-file-backup-geometry: settings/section/backup (sectionRun).
import { defineScenario } from '../../lib/scenario.mjs';
import * as S from '../../lib/settings.mjs';

export default defineScenario({
  id: 'settings/section-file-backup-geometry',
  covers: ["settings/section/backup"],
  expectFail: null,
  run(ctx) {
    return S.sectionRun(ctx, "File", "Backup", 'geometry');
  },
});
