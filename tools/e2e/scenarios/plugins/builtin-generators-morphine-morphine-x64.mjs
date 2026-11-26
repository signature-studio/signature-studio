// plugins/builtin-generators-morphine-morphine-x64: Morphine (generators, DLL Morphine_x64.dll (1745408 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-morphine-morphine-x64',
  covers: ['plugins/generators/morphine-morphine-x64', 'plugins/window/generators-morphine-morphine-x64', 'plugins/param/generators-morphine-morphine-x64', 'plugins/audio/generators-morphine-morphine-x64', 'plugins/stop/generators-morphine-morphine-x64'],
  expectFail: '#1473',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Morphine", kind: 'generators' });
  },
});
