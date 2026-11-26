// plugins/builtin-generators-ogun-ogun-x64: Ogun (generators, DLL Ogun_x64.dll (6188544 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-ogun-ogun-x64',
  covers: ['plugins/generators/ogun-ogun-x64', 'plugins/window/generators-ogun-ogun-x64', 'plugins/param/generators-ogun-ogun-x64', 'plugins/audio/generators-ogun-ogun-x64', 'plugins/stop/generators-ogun-ogun-x64'],
  expectFail: '#1473',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Ogun", kind: 'generators' });
  },
});
