// file/export-midi: File > Export > MIDI file (the item 4 of the sub-menu, from 0): the engine writes the file and the page lists it with a size above 0 and the name ends .mid.
import { defineScenario } from '../../lib/scenario.mjs';
import { exportCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/export-midi',
  covers: ['file/export/midi'],
  expectFail: 'new',
  async run(ctx) {
    const c = await exportCase(ctx, 4, ".mid");
    return { ok: c.ok, engineState: c.state, notes: c.notes, results: { bytes: c.file?.size ?? 0, name: c.file?.name ?? null } };
  },
});
