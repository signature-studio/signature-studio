// plugins/builtin-effects-frequency-splitter-frequency-splitter-x64: Frequency Splitter (effects, DLL Frequency Splitter_x64.dll (4386816 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-frequency-splitter-frequency-splitter-x64',
  covers: ['plugins/effects/frequency-splitter-frequency-splitter-x64', 'plugins/window/effects-frequency-splitter-frequency-splitter-x64', 'plugins/param/effects-frequency-splitter-frequency-splitter-x64', 'plugins/audio/effects-frequency-splitter-frequency-splitter-x64', 'plugins/stop/effects-frequency-splitter-frequency-splitter-x64'],
  expectFail: '#1437',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Frequency Splitter", kind: 'effects' });
  },
});
