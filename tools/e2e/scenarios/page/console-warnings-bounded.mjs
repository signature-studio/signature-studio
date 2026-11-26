// page/console-warnings-bounded: for eight seconds of idle the page's console writes at most 10 warnings (the number is the page's own, measured here as it is).
import { defineScenario } from '../../lib/scenario.mjs';
import { sleep } from '../../lib/flow.mjs';

const LIMIT = 10;

export default defineScenario({
  id: 'page/console-warnings-bounded',
  covers: ['page/console/warnings-bounded'],
  async run({ guest, page }) {
    const warns = [];
    const onConsole = (m) => { if (m.type() === 'warning') warns.push(m.text().slice(0, 120)); };
    page.on('console', onConsole);
    try {
      await sleep(8000);
    } finally {
      page.off('console', onConsole);
    }
    const state = (await guest.state())?.state ?? null;
    return { ok: warns.length <= LIMIT, engineState: state, notes: [`warnings in 8 s of idle: ${warns.length} (limit ${LIMIT})${warns.length ? `: ${warns.slice(0, 3).join(' | ')}` : ''}`], results: { warnings: warns.length, limit: LIMIT } };
  },
});
