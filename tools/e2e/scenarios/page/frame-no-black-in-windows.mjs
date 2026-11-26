// page/frame-no-black-in-windows: the engine's picture is drawn in its windows: at most 10 % of the 64 x 64 tiles whose centre lies in a window are all black.
import { defineScenario } from '../../lib/scenario.mjs';
import { blackInWindows } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/frame-no-black-in-windows',
  covers: ['page/frame/no-black-in-windows'],
  async run({ guest }) {
    const b = await blackInWindows(guest);
    const state = (await guest.state())?.state ?? null;
    return { ok: b.black <= b.total * 0.1 && state === 'engine-idle', engineState: state, notes: [`black tiles inside the windows: ${b.black} of ${b.total} (${(100 * b.black / b.total).toFixed(1)} %)`], results: { black: b.black, total: b.total } };
  },
});
