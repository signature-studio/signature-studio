// resilience/reset-five-times: File > New (Ctrl+N) five times in a row: the engine is idle after each, and the main form stays in the table.
import { defineScenario } from '../../lib/scenario.mjs';
import { MAIN_FORM, resetProject, windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'resilience/reset-five-times',
  covers: [],
  async run({ guest }) {
    const notes = [];
    let ok = true;
    for (let i = 1; i <= 5; i++) {
      await resetProject(guest);
      const st = await guest.state();
      const main = (await windowTable(guest)).top.some((w) => w.cls === MAIN_FORM);
      if (st.state !== 'engine-idle' || !main) ok = false;
      notes.push(`reset ${i}: ${st.state}, main form ${main}`);
    }
    return { ok, engineState: (await guest.state())?.state ?? null, notes };
  },
});
