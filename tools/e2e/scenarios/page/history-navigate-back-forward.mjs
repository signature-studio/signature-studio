// page/history-navigate-back-forward: a real navigation to the page again and back (two loads of the document): each load starts the engine again to idle, and the project opens.
import { defineScenario } from '../../lib/scenario.mjs';
import { startProject } from '../../lib/common.mjs';

export default defineScenario({
  id: 'page/history-navigate-back-forward',
  covers: ['page/history/navigate'],
  displays: ['dpr:1'],
  async run({ guest, page, origin }) {
    await page.goto(`${origin}/?e2e=history`, { waitUntil: 'load', timeout: 120000 });
    const a = await startProject(guest);
    const ea = await guest.state();
    await page.goBack({ waitUntil: 'load', timeout: 120000 });
    const b = await startProject(guest);
    const eb = await guest.state();
    const ok = a.state === 'engine-idle' && b.state === 'engine-idle' && a.opened === true && b.opened === true;
    const why = (s) => (s ? `error ${JSON.stringify(s.error)}, exit ${JSON.stringify(s.exit)}` : 'unreadable');
    return { ok, engineState: b.state, notes: [`navigated to the page again: state ${a.state}, opened ${a.opened} (${why(ea)}); back: state ${b.state}, opened ${b.opened} (${why(eb)})`], results: { a: [a.state, a.opened], b: [b.state, b.opened] } };
  },
});
