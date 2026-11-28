// file/save-new-version: a demo song is open; File > Save new version (the item 6 of the File drop-down, menus.tsv file-6): the engine writes a new version and the page lists it with a size above 0.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuPath } from '../../lib/common.mjs';
import { dataPath, openDataFile, writtenCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/save-new-version',
  covers: ['file/menu-item/file-6', 'file/save/new-version'],
  expectFail: 'new',
  async run(ctx) {
    const op = await openDataFile(ctx, dataPath(ctx.repo, 'Data\\Demo projects\\NewStuff.flp'));
    if (!op.opened) return { ok: false, engineState: op.state, notes: [`the demo did not open (state ${op.state})`] };
    const w = await writtenCase(ctx, () => menuPath(ctx.guest, ['File', '#6']), '.flp');
    return { ok: w.ok, engineState: w.state, notes: [`demo "${op.title}": ${w.notes[0]}`], results: { bytes: w.file?.size ?? 0, name: w.file?.name ?? null } };
  },
});
