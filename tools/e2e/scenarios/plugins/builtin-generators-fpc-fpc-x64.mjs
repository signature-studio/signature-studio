// plugins/builtin-generators-fpc-fpc-x64: FPC (generators, DLL FPC_x64.dll (6067200 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fpc-fpc-x64',
  covers: ['plugins/generators/fpc-fpc-x64', 'plugins/window/generators-fpc-fpc-x64', 'plugins/param/generators-fpc-fpc-x64', 'plugins/audio/generators-fpc-fpc-x64', 'plugins/stop/generators-fpc-fpc-x64'],
  expectFail: '#1467',
  async run({ guest }) {
    return pluginSteps(guest, { name: "FPC", kind: 'generators' });
  },
});
