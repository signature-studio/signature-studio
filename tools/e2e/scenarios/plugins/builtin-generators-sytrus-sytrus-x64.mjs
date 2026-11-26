// plugins/builtin-generators-sytrus-sytrus-x64: Sytrus (generators, DLL Sytrus_x64.dll (6269440 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-sytrus-sytrus-x64',
  covers: ['plugins/generators/sytrus-sytrus-x64', 'plugins/window/generators-sytrus-sytrus-x64', 'plugins/param/generators-sytrus-sytrus-x64', 'plugins/audio/generators-sytrus-sytrus-x64', 'plugins/stop/generators-sytrus-sytrus-x64'],
  expectFail: '#1484',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Sytrus", kind: 'generators' });
  },
});
