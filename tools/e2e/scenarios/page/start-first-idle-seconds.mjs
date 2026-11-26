// page/start-first-idle-seconds: the page measured the time of the start to the first idle (the beacon's startup block; the target is in the beacon); the value is a number of seconds.
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/start-first-idle-seconds',
  covers: ['page/start/first-idle-seconds'],
  async run({ guest, page }) {
    const s = await page.evaluate(() => window.__app.perf.snapshot('first-idle').startup);
    const state = (await guest.state())?.state ?? null;
    const sec = s.firstIdleSeconds;
    return { ok: typeof sec === 'number' && sec > 0, engineState: state, notes: [`first idle after ${sec} s (the target is ${s.targetSeconds} s; a measurement, not a limit here)`], results: { firstIdleSeconds: sec, targetSeconds: s.targetSeconds } };
  },
});
