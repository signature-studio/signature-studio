// resilience/escape-storm: Escape pressed ten times at once (a key the windows may take) leaves the engine idle and the main form in place.
import { defineScenario } from '../../lib/scenario.mjs';
import { MAIN_FORM, windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'resilience/escape-storm',
  covers: [],
  async run({ guest }) {
    await guest.focusDesktop();
    for (let i = 0; i < 10; i++) await guest.escape();
    const st = await guest.state();
    const main = (await windowTable(guest)).top.some((w) => w.cls === MAIN_FORM);
    return { ok: st.state === 'engine-idle' && main, engineState: st.state, notes: [`state ${st.state}, main form ${main}`] };
  },
});
