// resilience/idle-20s-no-stop: with nothing sent for 20 s the engine stays idle and does not stop (the runner counts the stops and the running time too).
import { defineScenario } from '../../lib/scenario.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'resilience/idle-20s-no-stop',
  covers: [],
  async run({ guest }) {
    await sleep(20000);
    const st = await guest.state();
    return { ok: st.state === 'engine-idle' && st.error === null, engineState: st.state, notes: [`state ${st.state} after 20 s idle; error ${st.error ?? 'none'}`] };
  },
});
