// plugins/builtin-generators-sawer-sawer-x64: Sawer (generators, DLL Sawer_x64.dll (4310016 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-sawer-sawer-x64',
  covers: ['plugins/generators/sawer-sawer-x64', 'plugins/window/generators-sawer-sawer-x64', 'plugins/param/generators-sawer-sawer-x64', 'plugins/audio/generators-sawer-sawer-x64', 'plugins/stop/generators-sawer-sawer-x64'],
  expectFail: '#1475',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Sawer", kind: 'generators' });
  },
});
