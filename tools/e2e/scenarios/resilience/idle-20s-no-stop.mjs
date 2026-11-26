// resilience/idle-20s-no-stop: with nothing sent for 20 s the engine stays idle and does not stop. The wait is by the time difference (checked every 2 s); the stops are counted.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { sleep } from '../../lib/flow.mjs';
import { stateName } from '../../lib/resilience.mjs';

const SPAN = 20000;

export default defineScenario({
  id: 'resilience/idle-20s-no-stop',
  covers: ['resilience/soak/idle-20s-no-stop'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const t0 = Date.now();
    const seen = {};
    let stops = 0;
    while (Date.now() - t0 < SPAN) {
      const s = await stateName(guest);
      seen[s] = (seen[s] ?? 0) + 1;
      if (s === 'error' || s === 'exited') { stops++; break; }
      await sleep(2000);
    }
    const st = await stateName(guest);
    const ms = Date.now() - t0;
    return { ok: stops === 0 && st === 'engine-idle', engineState: st, notes: [`${ms} ms with nothing sent: states ${JSON.stringify(seen)}, stops ${stops}`], results: { ms, states: seen, stops } };
  },
});
