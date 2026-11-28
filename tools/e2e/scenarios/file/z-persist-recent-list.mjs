// file/z-persist-recent-list: a demo song is opened, the page is reloaded: the Welcome window's recent module (TRecentFilesModule) should list the project.
// The Welcome window is read the way the start reads it: the usage box (No thanks) first, then the Welcome window (it comes after the box), and the module is looked for in the
// window table then. expectFail new: the Welcome window is shown after the reload, and the recent module is not in it. The mark #1409 (the write place) is closed; #1502 keeps
// the list across the reload (no stop after it); the missing module has no Issue of its own yet.
import { defineScenario } from '../../lib/scenario.mjs';
import { GDPR, startProject, waitWindow, windowTable } from '../../lib/common.mjs';
import { childrenOfClass } from '../../lib/dump.mjs';
import { noThanks, waitStable, WELCOME } from '../../lib/flow.mjs';
import { dataPath, openDataFile } from '../../lib/file.mjs';

export default defineScenario({
  id: 'file/z-persist-recent-list',
  covers: ['file/persist/recent-list', 'file/open-recent/welcome-list'],
  expectFail: 'new',
  async run(ctx) {
    const { guest, page, repo } = ctx;
    const op = await openDataFile(ctx, dataPath(repo, 'Data\\Demo projects\\NewStuff.flp'));
    await page.reload({ waitUntil: 'load', timeout: 120000 });
    const first = await waitStable(guest, 180000);
    let welcome = null;
    if (first.state === 'engine-idle') {
      if (childrenOfClass(await windowTable(guest), [GDPR]).some((c) => c.vis)) await noThanks(guest);
      welcome = await waitWindow(guest, WELCOME, 30000);
    }
    const t = first.state === 'engine-idle' ? await windowTable(guest) : { top: [], children: [] };
    const recent = [...t.top, ...t.children].filter((w) => w.cls === 'TRecentFilesModule');
    const kids = welcome ? [...new Set(t.children.filter((c) => c.parent === welcome.lid).map((c) => c.cls))].slice(0, 12) : [];
    await startProject(guest);
    return {
      ok: op.opened === true && recent.length > 0,
      engineState: first.state,
      notes: [`opened ${op.opened} (title "${op.title}"); after the reload (first state ${first.state}) the Welcome window ${welcome ? 'is shown' : 'is not shown'}, its child classes ${JSON.stringify(kids)}; the recent module ${recent.length > 0 ? `shows (${recent.map((w) => w.title).join(' | ')})` : 'is not in the window table'}`],
    };
  },
});
