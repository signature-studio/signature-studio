// file/export-wav: File > Export > Wave file (Ctrl+R in FL) (the item 0 of the sub-menu, from 0): the engine writes the file and the page lists it with a size above 0 and the name ends .wav.
import { defineScenario } from '../../lib/scenario.mjs';
import { exportCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-wav',
  covers: ['file/export/wav'],
  expectFail: 'new',
  async run(ctx) {
    const c = await exportCase(ctx, 0, ".wav");
    return { ok: c.ok, engineState: c.state, notes: c.notes, results: { bytes: c.file?.size ?? 0, name: c.file?.name ?? null } };
  },
});
