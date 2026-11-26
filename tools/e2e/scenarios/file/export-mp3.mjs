// file/export-mp3: File > Export > MP3 file (the item 1 of the sub-menu, from 0): the engine writes the file and the page lists it with a size above 0 and the name ends .mp3.
import { defineScenario } from '../../lib/scenario.mjs';
import { exportCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-mp3',
  covers: ['file/export/mp3'],
  expectFail: 'new',
  async run(ctx) {
    const c = await exportCase(ctx, 1, ".mp3");
    return { ok: c.ok, engineState: c.state, notes: c.notes, results: { bytes: c.file?.size ?? 0, name: c.file?.name ?? null } };
  },
});
