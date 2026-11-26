// plugins/builtin-effects-frequency-shifter-frequency-shifter-x64: Frequency Shifter (effects, DLL Frequency Shifter_x64.dll (3876864 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-frequency-shifter-frequency-shifter-x64',
  covers: ['plugins/effects/frequency-shifter-frequency-shifter-x64', 'plugins/window/effects-frequency-shifter-frequency-shifter-x64', 'plugins/param/effects-frequency-shifter-frequency-shifter-x64', 'plugins/audio/effects-frequency-shifter-frequency-shifter-x64', 'plugins/stop/effects-frequency-shifter-frequency-shifter-x64'],
  expectFail: '#1436',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Frequency Shifter", kind: 'effects' });
  },
});
