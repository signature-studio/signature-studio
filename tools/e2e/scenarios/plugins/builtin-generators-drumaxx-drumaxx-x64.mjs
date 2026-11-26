// plugins/builtin-generators-drumaxx-drumaxx-x64: Drumaxx (generators, DLL Drumaxx_x64.dll (4423168 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-drumaxx-drumaxx-x64',
  covers: ['plugins/generators/drumaxx-drumaxx-x64', 'plugins/window/generators-drumaxx-drumaxx-x64', 'plugins/param/generators-drumaxx-drumaxx-x64', 'plugins/audio/generators-drumaxx-drumaxx-x64', 'plugins/stop/generators-drumaxx-drumaxx-x64'],
  expectFail: '#1465',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Drumaxx", kind: 'generators' });
  },
});
