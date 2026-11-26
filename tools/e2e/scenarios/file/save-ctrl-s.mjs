// file/save-ctrl-s: a demo song is open; Save (Ctrl+S): the trial window is closed, the engine writes the project, and the page lists the written .flp with a size above 0.
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile, writtenCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/save-ctrl-s',
  covers: ['file/save/ctrl-s'],
  expectFail: 'new',
  async run(ctx) {
    const { guest, repo } = ctx;
    const op = await openDataFile(ctx, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'));
    if (!op.opened) return { ok: false, engineState: op.state, notes: [`the demo did not open (state ${op.state})`] };
    const w = await writtenCase(ctx, async () => {
      await guest.focusDesktop();
      await guest.ctrl('KeyS', 's', 83);
      return 'ctrl-s';
    }, '.flp');
    return { ok: w.ok, engineState: w.state, notes: [`demo "${op.title}": ${w.notes[0]}`], results: { bytes: w.file?.size ?? 0, name: w.file?.name ?? null } };
  },
});
