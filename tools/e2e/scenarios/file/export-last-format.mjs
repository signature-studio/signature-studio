// file/export-last-format: File > Export > Last exported format(s) (the item 6 of the sub-menu, from 0): the engine writes the file and the page lists it with a size above 0.
import { defineScenario } from '../../lib/scenario.mjs';
import { exportCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-last-format',
  covers: ['file/export/last-format'],
  expectFail: 'new',
  async run(ctx) {
    const c = await exportCase(ctx, 6, null);
    return { ok: c.ok, engineState: c.state, notes: c.notes, results: { bytes: c.file?.size ?? 0, name: c.file?.name ?? null } };
  },
});
