// plugins/builtin-generators-directwave-directwave-x64: DirectWave (generators, DLL DirectWave_x64.dll (1120256 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-directwave-directwave-x64',
  covers: ['plugins/generators/directwave-directwave-x64', 'plugins/window/generators-directwave-directwave-x64', 'plugins/param/generators-directwave-directwave-x64', 'plugins/audio/generators-directwave-directwave-x64', 'plugins/stop/generators-directwave-directwave-x64'],
  expectFail: '#1465',
  async run({ guest }) {
    return pluginSteps(guest, { name: "DirectWave", kind: 'generators' });
  },
});
