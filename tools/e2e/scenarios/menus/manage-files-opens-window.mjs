// menus/manage-files-opens-window: File > Manage files: the choice opens the OPFS file window (menus/manage-files-opens-window).
import { defineScenario } from '../../lib/scenario.mjs';
import { manageFilesOpens, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/manage-files-opens-window',
  covers: [],
  displays: ['4k-dpr1', '4k-dpr2'],
  expectFail: null,
  async run(ctx) {
    return withNote(await manageFilesOpens(ctx), "displays: 4k-dpr1 and 4k-dpr2 (the item is read at both)");
  },
});
