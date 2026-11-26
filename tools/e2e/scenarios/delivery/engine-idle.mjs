// delivery/engine-idle: the delivered bundle starts the real engine: the page reaches engine-idle with no error and no stand-in screen.
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'delivery/engine-idle',
  covers: ['delivery/boot/engine-idle'],
  displays: ['4k-dpr1'],
  async run({ guest }) {
    const s = await guest.state();
    const ok = s !== null && s.state === 'engine-idle' && s.error === null && s.standIn === false;
    return { ok, notes: [`state ${s?.state}, error ${s?.error ?? 'none'}, stand-in ${s?.standIn}`], engineState: s?.state ?? null };
  },
});
