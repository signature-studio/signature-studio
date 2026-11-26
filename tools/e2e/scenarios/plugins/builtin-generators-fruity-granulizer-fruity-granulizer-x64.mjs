// plugins/builtin-generators-fruity-granulizer-fruity-granulizer-x64: Fruity Granulizer (generators, DLL Fruity Granulizer_x64.dll (4475904 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-granulizer-fruity-granulizer-x64',
  covers: ['plugins/generators/fruity-granulizer-fruity-granulizer-x64', 'plugins/window/generators-fruity-granulizer-fruity-granulizer-x64', 'plugins/param/generators-fruity-granulizer-fruity-granulizer-x64', 'plugins/audio/generators-fruity-granulizer-fruity-granulizer-x64', 'plugins/stop/generators-fruity-granulizer-fruity-granulizer-x64'],
  expectFail: '#1469',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Granulizer", kind: 'generators' });
  },
});
