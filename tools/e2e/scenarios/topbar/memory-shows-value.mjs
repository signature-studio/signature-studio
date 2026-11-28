// topbar/memory-shows-value: the top bar's Memory Usage shows a value: its "<n> MB" label (the guest's memory) agrees with the engine's last K32GetProcessMemoryInfo answer.
import { defineScenario } from '../../lib/scenario.mjs';
import { memoryValue } from '../../lib/topbar.mjs';

export default defineScenario({
  id: 'topbar/memory-shows-value',
  covers: ['topbar/value/memory'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return memoryValue(guest);
  },
});
