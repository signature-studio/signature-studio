// page/start-reaches-idle: the start reaches the idle engine and the project opens (the state the page's start leaves).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/start-reaches-idle',
  covers: ['page/start/reaches-idle'],
  async run({ guest, page }) {
    const s = await guest.state();
    const title = await page.title();
    const ok = s !== null && s.state === 'engine-idle' && s.error === null && s.exit === null;
    return { ok, engineState: s?.state ?? null, notes: [`state ${s?.state}; error ${s?.error ?? 'none'}; title "${title}"`], results: { state: s?.state ?? null, title } };
  },
});
