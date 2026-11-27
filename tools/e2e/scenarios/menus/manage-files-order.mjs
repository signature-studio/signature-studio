// menus/manage-files-order: File > Manage files: the item is in the File drop-down between Revert to last backup and Exit (menus/manage-files-order).
import { defineScenario } from '../../lib/scenario.mjs';
import { manageFilesOrder, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/manage-files-order',
  covers: [],
  displays: ['4k-dpr1', '4k-dpr2'],
  expectFail: null,
  async run(ctx) {
    return withNote(await manageFilesOrder(ctx), "displays: 4k-dpr1 and 4k-dpr2 (the item is read at both)");
  },
});
