// file/export-ogg: File > Export > OGG file (the item 2 of the sub-menu, from 0): the engine writes the file and the page lists it with a size above 0 and the name ends .ogg.
import { defineScenario } from '../../lib/scenario.mjs';
import { exportCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-ogg',
  covers: ['file/export/ogg'],
  expectFail: 'new',
  async run(ctx) {
    const c = await exportCase(ctx, 2, ".ogg");
    return { ok: c.ok, engineState: c.state, notes: c.notes, results: { bytes: c.file?.size ?? 0, name: c.file?.name ?? null } };
  },
});
