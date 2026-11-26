// file/save-menu-save: a demo song is open; File > Save (the item 4 of the File drop-down, menus.tsv file-4): the engine writes the project and the page lists the .flp with a size above 0.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuPath } from '../../lib/common.mjs';
import { dataPath, openDataFile, writtenCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/save-menu-save',
  covers: ['file/menu-item/file-4'],
  expectFail: 'new',
  async run(ctx) {
    const op = await openDataFile(ctx, dataPath(ctx.repo, 'Data\\Demo projects\\NewStuff.flp'));
    if (!op.opened) return { ok: false, engineState: op.state, notes: [`the demo did not open (state ${op.state})`] };
    const w = await writtenCase(ctx, () => menuPath(ctx.guest, ['File', '#4']), '.flp');
    return { ok: w.ok, engineState: w.state, notes: [`demo "${op.title}": ${w.notes[0]}`], results: { bytes: w.file?.size ?? 0, name: w.file?.name ?? null } };
  },
});
