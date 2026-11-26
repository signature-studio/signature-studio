// file/open-menu-item: File > Open (the item 3 of the File drop-down, menus.tsv file-3): the open box shows, and the demo song given to it opens.
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { menuPath } from '../../lib/common.mjs';
import { dismissProblems } from '../../lib/flow.mjs';
import { dataPath, giveFiles, logSince, waitBox, waitOpened } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/open-menu-item',
  covers: ['file/menu-item/file-3'],
  async run(ctx) {
    const { guest, cdp, repo } = ctx;
    await dismissProblems(guest);
    const before = await guest.state();
    const logStart = (await guest.ev('window.__app.engineLog().length').catch(() => 0)) ?? 0;
    const id = await menuPath(guest, ['File', '#3']);
    const box = await waitBox(guest, 30000);
    if (box !== 'filedlg') {
      return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [`no file box after ${id} (box ${box})`] };
    }
    const abs = dataPath(repo, 'Data\\Demo projects\\NewStuff.flp');
    const g = await giveFiles(guest, cdp, [abs]);
    const end = await waitOpened(guest, { box, name: path.basename(abs), before, logStart, ms: 120000 });
    return {
      ok: g.given === true && end.opened === true,
      engineState: end.state,
      notes: [`${id}: box ${box}, the demo given ${g.given}, title "${end.title}" (before "${end.titleBefore}"), state ${end.state}, the name in the log ${end.logHit !== null} (${(await logSince(guest, logStart)).length} lines)`],
    };
  },
});
