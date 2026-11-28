// plugins/builtin-generators-plucked-plucked-x64: Plucked! (generators, DLL Plucked!_x64.dll (3697152 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-plucked-plucked-x64',
  covers: ['plugins/generators/plucked-plucked-x64', 'plugins/window/generators-plucked-plucked-x64', 'plugins/param/generators-plucked-plucked-x64', 'plugins/audio/generators-plucked-plucked-x64', 'plugins/stop/generators-plucked-plucked-x64'],
  expectFail: '#1474',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Plucked!", kind: 'generators' });
  },
});
