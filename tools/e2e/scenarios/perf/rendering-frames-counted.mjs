// perf/rendering-frames-counted: the page's requestAnimationFrame runs: at least 20 frames in 2 s (the number is the runner's; this runner has no GPU).
import { defineScenario } from '../../lib/scenario.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'perf/rendering-frames-counted',
  covers: [],
  async run({ guest }) {
    const a = await guest.ev('window.__e2eRaf ? window.__e2eRaf.n : 0');
    await sleep(2000);
    const b = await guest.ev('window.__e2eRaf ? window.__e2eRaf.n : 0');
    const frames = b - a;
    return { ok: frames >= 20, engineState: (await guest.state())?.state ?? null, notes: [`${frames} frames in 2 s (runner value; not this machine)`] };
  },
});
