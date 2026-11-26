// file/export-flac: File > Export > FLAC file (the item 3 of the sub-menu, from 0): the engine writes the file and the page lists it with a size above 0 and the name ends .flac.
import { defineScenario } from '../../lib/scenario.mjs';
import { exportCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-flac',
  covers: ['file/export/flac'],
  expectFail: 'new',
  async run(ctx) {
    const c = await exportCase(ctx, 3, ".flac");
    return { ok: c.ok, engineState: c.state, notes: c.notes, results: { bytes: c.file?.size ?? 0, name: c.file?.name ?? null } };
  },
});
