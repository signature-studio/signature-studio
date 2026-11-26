// plugins/builtin-generators-slicex-slicex-x64: Slicex (generators, DLL Slicex_x64.dll (6282240 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-slicex-slicex-x64',
  covers: ['plugins/generators/slicex-slicex-x64', 'plugins/window/generators-slicex-slicex-x64', 'plugins/param/generators-slicex-slicex-x64', 'plugins/audio/generators-slicex-slicex-x64', 'plugins/stop/generators-slicex-slicex-x64'],
  expectFail: '#1476',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Slicex", kind: 'generators' });
  },
});
