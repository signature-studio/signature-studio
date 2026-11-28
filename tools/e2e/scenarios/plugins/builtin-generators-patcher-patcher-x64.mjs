// plugins/builtin-generators-patcher-patcher-x64: Patcher (generators, DLL Patcher_x64.dll (7052800 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-patcher-patcher-x64',
  covers: ['plugins/generators/patcher-patcher-x64', 'plugins/window/generators-patcher-patcher-x64', 'plugins/param/generators-patcher-patcher-x64', 'plugins/audio/generators-patcher-patcher-x64', 'plugins/stop/generators-patcher-patcher-x64'],
  expectFail: '#1473',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Patcher", kind: 'generators' });
  },
});
