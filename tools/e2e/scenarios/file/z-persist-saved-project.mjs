// file/z-persist-saved-project: a demo song is saved (Ctrl+S: the page lists the written .flp), the page is reloaded: the saved project's name is in the guest's memory after the
// start (the written file came back; the engine's close of the file hands it to the page's storage, see tools/persist).
import { defineScenario } from '../../lib/scenario.mjs';
import { guestText, persist } from '../../lib/common.mjs';
import { dataPath, openDataFile, writtenCase } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/z-persist-saved-project',
  covers: ['file/persist/saved-project'],
  expectFail: 'new',
  async run(ctx) {
    const { guest, page, repo } = ctx;
    const op = await openDataFile(ctx, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'));
    if (!op.opened) return { ok: false, engineState: op.state, notes: [`the demo did not open (state ${op.state})`] };
    const w = await writtenCase(ctx, async () => {
      await guest.focusDesktop();
      await guest.ctrl('KeyS', 's', 83);
      return 'ctrl-s';
    }, '.flp');
    if (!w.ok) return { ok: false, engineState: w.state, notes: [`the save did not write: ${w.notes[0]}`] };
    const stem = w.file.name.replace(/\.flp$/i, '');
    const p = await persist(page, guest);
    const t = await guestText(guest, [stem], 0);
    return {
      ok: p.opened === true && t.lines.length > 0,
      engineState: p.state,
      notes: [`saved ${w.file.name} (${w.file.size} B); after the reload: state ${p.state}, opened ${p.opened}; "${stem}" in the guest's memory: ${t.lines.length} line(s)`],
    };
  },
});
