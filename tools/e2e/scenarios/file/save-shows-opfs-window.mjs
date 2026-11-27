// file/save-shows-opfs-window: a demo project is open; File > Save as (File > #5) shows the engine's Save window (the page's file manager in selection mode, with its File name box
// #fx-filename). A name is typed and Save is pressed (#fx-ok); the engine writes the project, the page lists the written file, and the file is in the browser's storage (OPFS)
// at the place the person chose (Issue #1519). No browser chooser or download is used for it.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuPath } from '../../lib/common.mjs';
import { clickElement, dataPath, dialogState, openDataFile, writeAndWait } from '../../lib/file.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'file/save-shows-opfs-window',
  covers: ['file/save/opfs-window'],
  async run(ctx) {
    const { guest, repo, page } = ctx;
    let choosers = 0;
    const onChooser = () => { choosers++; };
    page.on('filechooser', onChooser);
    try {
      const op = await openDataFile(ctx, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'));
      if (!op.opened) return { ok: false, engineState: op.state, notes: [`the demo did not open (state ${op.state})`] };
      const name = `opfs-save-${Date.now() % 1000000}.flp`;
      let shown = false;
      let id = null;
      const r = await writeAndWait(ctx, async () => {
        id = await menuPath(guest, ['File', '#5']);
        const t0 = Date.now();
        while (Date.now() - t0 < 15000) {
          if ((await dialogState(guest)).open) { shown = true; break; }
          await sleep(300);
        }
        if (!shown) throw new Error(`${id}: no Save window`);
        await guest.ev(`(() => { const i = document.getElementById('fx-filename'); if (!i) return false; i.value = ${JSON.stringify(name)}; i.dispatchEvent(new Event('input')); return true; })()`);
        await clickElement(guest, '#fx-ok');
        return id;
      });
      const listed = r.saved.find((f) => f.name === name) ?? null;
      const inStore = await guest.ev(`(async () => { try { const root = await navigator.storage.getDirectory(); const f = await (await root.getFileHandle(${JSON.stringify(name)})).getFile(); return f.size; } catch (e) { return -1; } })()`);
      const st = (await guest.state())?.state ?? null;
      return {
        ok: shown && listed !== null && listed.size > 0 && inStore === listed.size && choosers === 0 && st === 'engine-idle',
        engineState: st,
        notes: [`${id ?? 'File > #5'}: the Save window shown ${shown}; ${name} listed ${listed ? `${listed.size} B` : 'no'}; in the browser's storage ${inStore} B; browser chooser ${choosers}; trial window ${r.shop}; state ${st}${r.error ? `; error ${r.error}` : ''}`],
      };
    } finally {
      page.off('filechooser', onChooser);
    }
  },
});
