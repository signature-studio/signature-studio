// plugins/builtin-generators-rewired-rewired-x64: ReWired (generators, DLL ReWired_x64.dll (3991552 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-rewired-rewired-x64',
  covers: ['plugins/generators/rewired-rewired-x64', 'plugins/window/generators-rewired-rewired-x64', 'plugins/param/generators-rewired-rewired-x64', 'plugins/audio/generators-rewired-rewired-x64', 'plugins/stop/generators-rewired-rewired-x64'],
  expectFail: '#1474',
  async run({ guest }) {
    return pluginSteps(guest, { name: "ReWired", kind: 'generators' });
  },
});
