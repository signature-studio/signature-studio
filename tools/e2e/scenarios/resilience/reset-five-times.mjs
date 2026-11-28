// resilience/reset-five-times: File > New (Ctrl+N) five times in a row: the engine is idle after each, and the main form stays in the table.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { resetProject } from '../../lib/common.mjs';
import { mainFormHere, stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/reset-five-times',
  covers: ['resilience/project/new-repeat'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const notes = [];
    const rounds = [];
    for (let i = 1; i <= 5; i++) {
      await resetProject(guest);
      const st = await stateName(guest);
      const main = await mainFormHere(guest);
      rounds.push({ i, state: st, main });
      notes.push(`reset ${i}: ${st}, main form ${main}`);
    }
    const ok = rounds.every((r) => r.state === 'engine-idle' && r.main);
    return { ok, engineState: await stateName(guest), notes, results: { rounds: rounds.length, idle: rounds.filter((r) => r.state === 'engine-idle').length } };
  },
});
