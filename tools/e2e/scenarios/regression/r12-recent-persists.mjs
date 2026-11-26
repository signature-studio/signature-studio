// R12 (#1408) recent-persists. Report (L-063, #1385): the projects opened with File > Open stay in the Recent list after the page is reloaded (the engine is built
// again). Expectation: a project file with a name of its own (a copy of the installer's demo, named e2e-recent-<random>.flp, so that no other list holds the name)
// is opened with File > Open; its name is in the engine's memory (the Recent list); after a reload of the page the same name is in the engine's memory again,
// before anything is opened in the new run.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { openProjectFile, waitProjectOpen, waitStable } from '../../lib/flow.mjs';
import { startDefault, result } from './common.mjs';
import { DEMO } from './r03-open-demo-project.mjs';

const SCENARIO = {
  name: 'recent-persists',
  report: 'R12',
  expect: 'File > Open of a uniquely named copy of a demo project; after a reload of the page its name is in the engine memory (the Recent list) without opening it again',
  // the Recent list is not kept across a reload (#1502; #1409 and #1385 are closed)
  expectFail: '#1502',
};

const holds = (guest, mark) => guest.ev(`window.__app.guestText([${JSON.stringify(mark)}], 0).then((g) => g.best !== null && g.best !== undefined)`);

export default {
  ...SCENARIO,
  async run({ guest, page, cdp, repo, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const dir = path.join(repo, '.tmp-e2e-recent');
    const mark = `e2e-recent-${crypto.randomBytes(4).toString('hex')}.flp`;
    fs.mkdirSync(dir, { recursive: true });
    const copy = path.join(dir, mark);
    try {
      fs.copyFileSync(path.join(repo, 'lift', 'out-app', 'host', 'resources', DEMO), copy);
      await openProjectFile(guest, cdp, copy, log);
      const opened = await waitProjectOpen(guest, start.title, 240000);
      if (!opened.opened) return result(false, opened.state, [...start.notes, 'the project did not open']);
      const sanity = await holds(guest, mark);
      await page.reload({ waitUntil: 'load', timeout: 120000 });
      // read at the new run's first idle, before anything is opened in it (the default project is not opened here)
      const first = await waitStable(guest, 180000);
      const after = first.state === 'engine-idle' ? await holds(guest, mark) : false;
      const notes = [...start.notes, `"${mark}" after the open in the engine memory: ${sanity}`, `after the reload (first state ${first.state}): ${after}`];
      log(notes.at(-1));
      return result(sanity && after, first.state, notes, { sanity, after });
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  },
};
