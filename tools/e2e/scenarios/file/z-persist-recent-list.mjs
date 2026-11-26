// file/z-persist-recent-list (expectFail #1409): a demo song is opened, the page is reloaded: the Welcome window's recent module (TRecentFilesModule) should list the project.
// In this build FL does not write its recent list where the page keeps it (#1409: the write place is not found), so the module is not shown with the project. The mark is #1409.
import { defineScenario } from '../../lib/scenario.mjs';
import { startProject, windowTable } from '../../lib/common.mjs';
import { waitStable } from '../../lib/flow.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/z-persist-recent-list',
  covers: ['file/persist/recent-list', 'file/open-recent/welcome-list'],
  expectFail: '#1409',
  async run(ctx) {
    const { guest, page, repo } = ctx;
    const op = await openDataFile(ctx, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'));
    await page.reload({ waitUntil: 'load', timeout: 120000 });
    const first = await waitStable(guest, 180000);
    const t = first.state === 'engine-idle' ? await windowTable(guest) : { top: [], children: [] };
    const recent = [...t.top, ...t.children].filter((w) => w.cls === 'TRecentFilesModule');
    await startProject(guest);
    return {
      ok: op.opened === true && recent.length > 0,
      engineState: first.state,
      notes: [`opened ${op.opened} (title "${op.title}"); after the reload the recent module ${recent.length > 0 ? `shows (${recent.map((w) => w.title).join(' | ')})` : 'is not in the window table'}`],
    };
  },
});
