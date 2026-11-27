// file/open-demo-via-opfs: the installer's demo song is put into the engine's Open window (File > Open, Ctrl+O; Issue #1519): the window's Upload takes it into the browser's
// storage (OPFS), it is chosen in the window's list and Open is pressed. The song's title changes, the engine stays idle with no stop, and no browser chooser opened.
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

const DEMO = path.join('Data', 'Demo projects', 'Demo songs', 'DEDZII - NIGHT TERROR.flp');
// another demo song first: a song that is open already (an earlier scenario of the page opens the same one) would not change the title when it is opened again
const FIRST = 'Data\\Demo projects\\NewStuff.flp';

export default defineScenario({
  id: 'file/open-demo-via-opfs',
  covers: ['file/open/opfs-demo'],
  displays: ['4k-dpr1', '4k-dpr2'],
  async run(ctx) {
    const { guest, page } = ctx;
    let choosers = 0;
    const onChooser = () => { choosers++; };
    page.on('filechooser', onChooser);
    try {
      const first = await openDataFile(ctx, dataPath(ctx.repo, FIRST));
      const r = await openDataFile(ctx, dataPath(ctx.repo, DEMO));
      const inStore = await guest.ev(`(async () => { try { const root = await navigator.storage.getDirectory(); const f = await (await root.getFileHandle(${JSON.stringify(path.basename(DEMO))})).getFile(); return f.size; } catch (e) { return -1; } })()`);
      const st = (await guest.state())?.state ?? null;
      return {
        ok: first.opened === true && r.box === 'explorer' && r.opened === true && choosers === 0 && inStore > 0 && st === 'engine-idle',
        engineState: st,
        notes: [`NewStuff opened first ${first.opened} (title "${first.title}"); window ${r.box}; the song in the browser's storage ${inStore} B; browser chooser ${choosers}; title "${r.title}" (before "${r.titleBefore}"); open ${r.opened}; state ${st}`],
        results: { titleChanged: r.titleChanged, waitedMs: r.waitedMs },
      };
    } finally {
      page.off('filechooser', onChooser);
    }
  },
});
