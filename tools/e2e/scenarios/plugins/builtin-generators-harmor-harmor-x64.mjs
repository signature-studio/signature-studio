// plugins/builtin-generators-harmor-harmor-x64: Harmor (generators, DLL Harmor_x64.dll (6478336 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-harmor-harmor-x64',
  covers: ['plugins/generators/harmor-harmor-x64', 'plugins/window/generators-harmor-harmor-x64', 'plugins/param/generators-harmor-harmor-x64', 'plugins/audio/generators-harmor-harmor-x64', 'plugins/stop/generators-harmor-harmor-x64'],
  expectFail: '#1472',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Harmor", kind: 'generators' });
  },
});
