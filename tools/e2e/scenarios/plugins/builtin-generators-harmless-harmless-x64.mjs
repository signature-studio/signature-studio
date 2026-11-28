// plugins/builtin-generators-harmless-harmless-x64: Harmless (generators, DLL Harmless_x64.dll (4776448 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-harmless-harmless-x64',
  covers: ['plugins/generators/harmless-harmless-x64', 'plugins/window/generators-harmless-harmless-x64', 'plugins/param/generators-harmless-harmless-x64', 'plugins/audio/generators-harmless-harmless-x64', 'plugins/stop/generators-harmless-harmless-x64'],
  expectFail: '#1471',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Harmless", kind: 'generators' });
  },
});
