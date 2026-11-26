// plugins/builtin-generators-3x-osc-3x-osc-x64: 3x Osc (generators, DLL 3x Osc_x64.dll (4348928 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-3x-osc-3x-osc-x64',
  covers: ['plugins/generators/3x-osc-3x-osc-x64', 'plugins/window/generators-3x-osc-3x-osc-x64', 'plugins/param/generators-3x-osc-3x-osc-x64', 'plugins/audio/generators-3x-osc-3x-osc-x64', 'plugins/stop/generators-3x-osc-3x-osc-x64'],
  expectFail: '#1463',
  async run({ guest }) {
    return pluginSteps(guest, { name: "3x Osc", kind: 'generators' });
  },
});
